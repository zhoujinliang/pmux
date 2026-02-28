#!/usr/bin/env python3
"""
tests/helpers/analyze_performance.py
Analyze performance logs and generate reports.
"""

import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from typing import List, Dict, Optional
import json


@dataclass
class PerformanceMetric:
    name: str
    values: List[float]
    unit: str = "ms"


def parse_paint_times(log_content: str) -> PerformanceMetric:
    """Extract paint times from log content."""
    pattern = r'paint took (\d+)ms'
    matches = re.findall(pattern, log_content)
    values = [float(m) for m in matches]
    return PerformanceMetric("paint_time", values, "ms")


def parse_cache_stats(log_content: str) -> Dict[str, int]:
    """Extract cache hit/miss statistics."""
    hits = len(re.findall(r'cache hit', log_content, re.IGNORECASE))
    misses = len(re.findall(r'cache miss', log_content, re.IGNORECASE))
    return {"hits": hits, "misses": misses}


def parse_resize_events(log_content: str) -> List[Dict]:
    """Extract resize event information."""
    pattern = r'resize.*?(\d+)x(\d+)'
    matches = re.findall(pattern, log_content, re.IGNORECASE)
    return [{"cols": int(m[0]), "rows": int(m[1])} for m in matches]


def calculate_percentile(values: List[float], percentile: float) -> float:
    """Calculate the given percentile of a list of values."""
    if not values:
        return 0.0
    sorted_values = sorted(values)
    index = int(len(sorted_values) * percentile / 100)
    return sorted_values[min(index, len(sorted_values) - 1)]


def analyze_log_file(log_path: str) -> Dict:
    """Analyze a log file and return metrics."""
    with open(log_path, 'r') as f:
        content = f.read()
    
    paint_metric = parse_paint_times(content)
    cache_stats = parse_cache_stats(content)
    resize_events = parse_resize_events(content)
    
    return {
        "paint": {
            "count": len(paint_metric.values),
            "avg": sum(paint_metric.values) / len(paint_metric.values) if paint_metric.values else 0,
            "min": min(paint_metric.values) if paint_metric.values else 0,
            "max": max(paint_metric.values) if paint_metric.values else 0,
            "p50": calculate_percentile(paint_metric.values, 50),
            "p95": calculate_percentile(paint_metric.values, 95),
            "p99": calculate_percentile(paint_metric.values, 99),
        },
        "cache": cache_stats,
        "resize_events": len(resize_events),
    }


def print_report(analysis: Dict):
    """Print a formatted report."""
    print("=== Performance Analysis Report ===\n")
    
    # Paint timing
    paint = analysis["paint"]
    print("Paint Timing:")
    print(f"  Total paints:   {paint['count']}")
    print(f"  Average:        {paint['avg']:.2f}ms")
    print(f"  Min:            {paint['min']:.2f}ms")
    print(f"  Max:            {paint['max']:.2f}ms")
    print(f"  P50:            {paint['p50']:.2f}ms")
    print(f"  P95:            {paint['p95']:.2f}ms")
    print(f"  P99:            {paint['p99']:.2f}ms")
    print()
    
    # Cache stats
    cache = analysis["cache"]
    total_cache = cache["hits"] + cache["misses"]
    hit_rate = (cache["hits"] / total_cache * 100) if total_cache > 0 else 0
    print("Cache Statistics:")
    print(f"  Hits:           {cache['hits']}")
    print(f"  Misses:         {cache['misses']}")
    print(f"  Hit Rate:       {hit_rate:.1f}%")
    print()
    
    # Resize events
    print(f"Resize Events:    {analysis['resize_events']}")
    print()
    
    # Pass/Fail
    print("=== Acceptance Criteria ===")
    passed = True
    
    if paint['p95'] < 16:
        print(f"✓ PASS: P95 paint time ({paint['p95']:.2f}ms) < 16ms")
    else:
        print(f"✗ FAIL: P95 paint time ({paint['p95']:.2f}ms) >= 16ms")
        passed = False
    
    if hit_rate >= 90 or total_cache == 0:
        print(f"✓ PASS: Cache hit rate ({hit_rate:.1f}%) >= 90%")
    else:
        print(f"✗ FAIL: Cache hit rate ({hit_rate:.1f}%) < 90%")
        passed = False
    
    print()
    if passed:
        print("=== ALL CHECKS PASSED ===")
    else:
        print("=== SOME CHECKS FAILED ===")
    
    return passed


def main():
    if len(sys.argv) < 2:
        print("Usage: python analyze_performance.py <log_file>")
        print("       python analyze_performance.py --json <log_file>")
        sys.exit(1)
    
    json_output = "--json" in sys.argv
    log_path = sys.argv[-1]
    
    try:
        analysis = analyze_log_file(log_path)
        
        if json_output:
            print(json.dumps(analysis, indent=2))
        else:
            passed = print_report(analysis)
            sys.exit(0 if passed else 1)
            
    except FileNotFoundError:
        print(f"Error: File not found: {log_path}")
        sys.exit(1)
    except Exception as e:
        print(f"Error analyzing log: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()