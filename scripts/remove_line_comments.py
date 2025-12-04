#!/usr/bin/env python3
"""
Remove unnecessary inline comments from Rust source files.
Preserves:
- File-level doc comments (//!)
- Module/item doc comments (///)
- Complex function documentation
- TODO/FIXME/NOTE markers
"""

import re
import sys
from pathlib import Path
from typing import List, Tuple

# Patterns to preserve
PRESERVE_PATTERNS = [
    r'^\s*//!',           # File-level docs
    r'^\s*///',           # Item docs
    r'^\s*//\s*TODO',     # TODO markers
    r'^\s*//\s*FIXME',    # FIXME markers
    r'^\s*//\s*NOTE',     # NOTE markers
    r'^\s*//\s*SAFETY',   # Safety comments
    r'^\s*//\s*HACK',     # Hack explanations
    r'^\s*//\s*XXX',      # XXX markers
]

def should_preserve_comment(line: str) -> bool:
    """Check if a comment line should be preserved."""
    for pattern in PRESERVE_PATTERNS:
        if re.match(pattern, line, re.IGNORECASE):
            return True
    return False

def is_simple_inline_comment(line: str) -> bool:
    """Detect simple inline comments that explain obvious code."""
    # Match lines with code followed by // comment
    if '//' not in line:
        return False
    
    # Don't touch doc comments or preserved patterns
    if should_preserve_comment(line):
        return False
    
    # Check if it's a trailing comment (code // comment)
    code_part = line.split('//')[0].strip()
    if code_part:  # Has code before comment
        return True
    
    return False

def remove_inline_comments(content: str) -> Tuple[str, int]:
    """Remove simple inline comments from Rust code."""
    lines = content.split('\n')
    modified_lines = []
    removed_count = 0
    
    for line in lines:
        # Preserve empty lines and non-comment lines
        if not line.strip() or '//' not in line:
            modified_lines.append(line)
            continue
        
        # Preserve important comments
        if should_preserve_comment(line):
            modified_lines.append(line)
            continue
        
        # Remove simple inline comments
        if is_simple_inline_comment(line):
            code_part = line.split('//')[0].rstrip()
            modified_lines.append(code_part)
            removed_count += 1
        else:
            # Standalone comment line - check if it's trivial
            comment_text = line.split('//', 1)[1].strip().lower()
            
            # Remove very short or obvious comments
            trivial_phrases = [
                'update', 'set', 'get', 'return', 'create', 'init',
                'check', 'validate', 'handle', 'process', 'load'
            ]
            
            if len(comment_text) < 15 or any(phrase in comment_text for phrase in trivial_phrases):
                # Skip this comment line if it's standalone
                if not line.split('//')[0].strip():
                    removed_count += 1
                    continue
            
            modified_lines.append(line)
    
    return '\n'.join(modified_lines), removed_count

def process_file(file_path: Path, dry_run: bool = False) -> Tuple[bool, int]:
    """Process a single Rust file."""
    try:
        content = file_path.read_text(encoding='utf-8')
        new_content, removed = remove_inline_comments(content)
        
        if removed > 0:
            if not dry_run:
                file_path.write_text(new_content, encoding='utf-8')
            print(f"{'[DRY RUN] ' if dry_run else ''}Processed {file_path}: removed {removed} comments")
            return True, removed
        
        return False, 0
    except Exception as e:
        print(f"Error processing {file_path}: {e}", file=sys.stderr)
        return False, 0

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description='Remove unnecessary inline comments from Rust files')
    parser.add_argument('paths', nargs='*', default=['src'], help='Paths to process (default: src)')
    parser.add_argument('--dry-run', action='store_true', help='Show what would be changed without modifying files')
    parser.add_argument('--exclude', action='append', default=[], help='Patterns to exclude')
    
    args = parser.parse_args()
    
    total_files = 0
    total_removed = 0
    modified_files = 0
    
    for path_str in args.paths:
        path = Path(path_str)
        
        if path.is_file() and path.suffix == '.rs':
            files = [path]
        elif path.is_dir():
            files = path.rglob('*.rs')
        else:
            print(f"Skipping {path}: not a Rust file or directory")
            continue
        
        for file_path in files:
            # Check exclusions
            if any(excl in str(file_path) for excl in args.exclude):
                continue
            
            total_files += 1
            modified, removed = process_file(file_path, args.dry_run)
            
            if modified:
                modified_files += 1
                total_removed += removed
    
    print(f"\n{'[DRY RUN] ' if args.dry_run else ''}Summary:")
    print(f"  Files scanned: {total_files}")
    print(f"  Files modified: {modified_files}")
    print(f"  Comments removed: {total_removed}")

if __name__ == '__main__':
    main()
