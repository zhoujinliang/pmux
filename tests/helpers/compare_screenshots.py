#!/usr/bin/env python3
"""
tests/helpers/compare_screenshots.py
Compare screenshots for visual regression testing.
"""

import sys
from pathlib import Path

try:
    from PIL import Image
    import numpy as np
    HAS_PIL = True
except ImportError:
    HAS_PIL = False


def compare_images_pixel(img1_path: str, img2_path: str, threshold: float = 0.01) -> dict:
    """
    Compare two images pixel by pixel.
    Returns dict with match result and diff percentage.
    """
    if not HAS_PIL:
        return {"error": "PIL not installed. Run: pip install Pillow numpy"}
    
    img1 = Image.open(img1_path)
    img2 = Image.open(img2_path)
    
    # Convert to same mode
    if img1.mode != img2.mode:
        img2 = img2.convert(img1.mode)
    
    # Resize if needed
    if img1.size != img2.size:
        img2 = img2.resize(img1.size)
    
    # Convert to numpy arrays
    arr1 = np.array(img1)
    arr2 = np.array(img2)
    
    # Calculate difference
    diff = np.abs(arr1.astype(float) - arr2.astype(float))
    total_pixels = arr1.size
    diff_pixels = np.sum(diff > 0)
    diff_percentage = diff_pixels / total_pixels
    
    return {
        "match": diff_percentage <= threshold,
        "diff_percentage": diff_percentage * 100,
        "diff_pixels": int(diff_pixels),
        "total_pixels": total_pixels,
    }


def compare_images_simple(img1_path: str, img2_path: str) -> dict:
    """Simple comparison using file size and basic stats."""
    import hashlib
    
    with open(img1_path, 'rb') as f:
        hash1 = hashlib.md5(f.read()).hexdigest()
    
    with open(img2_path, 'rb') as f:
        hash2 = hashlib.md5(f.read()).hexdigest()
    
    size1 = Path(img1_path).stat().st_size
    size2 = Path(img2_path).stat().st_size
    
    return {
        "match": hash1 == hash2,
        "hash1": hash1,
        "hash2": hash2,
        "size1": size1,
        "size2": size2,
        "size_diff": abs(size1 - size2),
    }


def main():
    if len(sys.argv) < 3:
        print("Usage: python compare_screenshots.py <image1> <image2> [threshold]")
        print("       threshold: 0.0-1.0, default 0.01 (1%)")
        sys.exit(1)
    
    img1_path = sys.argv[1]
    img2_path = sys.argv[2]
    threshold = float(sys.argv[3]) if len(sys.argv) > 3 else 0.01
    
    for path in [img1_path, img2_path]:
        if not Path(path).exists():
            print(f"Error: File not found: {path}")
            sys.exit(1)
    
    if HAS_PIL:
        result = compare_images_pixel(img1_path, img2_path, threshold)
        print(f"Diff: {result['diff_percentage']:.2f}% ({result['diff_pixels']}/{result['total_pixels']} pixels)")
    else:
        result = compare_images_simple(img1_path, img2_path)
        print(f"Hash match: {result['match']}")
        print(f"Size diff: {result['size_diff']} bytes")
    
    if result.get("match", False):
        print("✓ PASS: Images match")
        sys.exit(0)
    else:
        print("✗ FAIL: Images differ")
        sys.exit(1)


if __name__ == "__main__":
    main()