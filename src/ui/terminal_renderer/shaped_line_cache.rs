//! ShapedLineCache: LRU cache for shaped text to avoid repeated shape_line (Harfbuzz) calls.
//! Key = (text, font, font_size, style_hash). Required for vim/scroll performance.

use gpui::{Font, Pixels, TextRun};
use lru::LruCache;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;

/// Key for the ShapedLine cache. Captures (text, font, font_size, style).
#[derive(Clone, Debug)]
pub struct CacheKey {
    text: String,
    font: Font,
    font_size: Pixels,
    style_hash: u64,
}

impl CacheKey {
    pub fn new(text: &str, font: &Font, font_size: Pixels, style: &TextRun) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        style.color.hash(&mut hasher);
        style.background_color.hash(&mut hasher);
        style.underline.hash(&mut hasher);
        style.strikethrough.hash(&mut hasher);
        Self {
            text: text.to_string(),
            font: font.clone(),
            font_size,
            style_hash: hasher.finish(),
        }
    }
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
        self.font.hash(state);
        f32::from(self.font_size).to_bits().hash(state);
        self.style_hash.hash(state);
    }
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.font == other.font
            && self.font_size == other.font_size
            && self.style_hash == other.style_hash
    }
}

impl Eq for CacheKey {}

/// LRU cache for shaped lines. get_or_insert returns cached value on hit, otherwise computes and stores.
pub struct ShapedLineCache<V> {
    cache: LruCache<CacheKey, Arc<V>>,
}

impl<V> ShapedLineCache<V> {
    pub fn new(cap: usize) -> Self {
        let cap = NonZeroUsize::new(cap.max(1)).unwrap();
        Self {
            cache: LruCache::new(cap),
        }
    }

    /// Get cached value or insert by calling `f`. On cache hit, `f` is not called.
    pub fn get_or_insert<F>(&mut self, key: &CacheKey, f: F) -> Arc<V>
    where
        F: FnOnce() -> V,
    {
        if let Some(entry) = self.cache.get(key) {
            return Arc::clone(entry);
        }
        let value = Arc::new(f());
        self.cache.put(key.clone(), Arc::clone(&value));
        value
    }

    /// Clear the cache. Use when GPU context is lost or repeated paint failures.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{font, px};

    #[test]
    fn test_shaped_line_cache_hit_avoids_insert_call() {
        let mut cache: ShapedLineCache<String> = ShapedLineCache::new(1000);
        let style = TextRun::default();
        let key = CacheKey::new("hello", &font("Menlo"), px(14.), &style);

        let mut insert_count = 0u32;
        let shaped1 = cache.get_or_insert(&key, || {
            insert_count += 1;
            "shaped_hello".to_string()
        });
        let shaped2 = cache.get_or_insert(&key, || {
            insert_count += 1;
            panic!("should not call on cache hit");
        });

        assert_eq!(insert_count, 1, "insert closure should run only once");
        assert_eq!(shaped1.as_ref(), "shaped_hello");
        assert_eq!(shaped2.as_ref(), "shaped_hello");
        assert!(
            std::ptr::eq(shaped1.as_ref(), shaped2.as_ref()),
            "cache hit should return same Arc allocation"
        );
    }
}
