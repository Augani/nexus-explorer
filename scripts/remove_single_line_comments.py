#!/usr/bin/env python3
"""
Remove single-line comments from Rust files.

Preserves:
- Doc comments (/// and //!)
- File-level block comments at the top (/* ... */)
- Comments inside string literals
- Multi-line block comments

Usage:
    python scripts/remove_single_line_comments.py [--dry-run] [path]
    
    --dry-run: Show what would be changed without modifying files
    path: Specific file or directory (defaults to src/)
"""

import os
import re
import sys
import argparse
from pathlib import Path


def is_inside_string(line: str, comment_pos: int) -> bool:
    """Check if the comment position is inside a string literal."""
    in_string = False
    string_char = None
    i = 0
    
    while i < comment_pos:
        char = line[i]
        
        # Handle escape sequences
        if i > 0 and line[i-1] == '\\':
            i += 1
            continue
            
        # Handle raw strings r"..." or r#"..."#
        if char == 'r' and i + 1 < len(line) and line[i+1] in '"#':
            # Skip raw string detection for simplicity - treat as regular
            pass
        
        if char in '"\'':
            if not in_string:
                in_string = True
                string_char = char
            elif char == string_char:
                in_string = False
                string_char = None
        
        i += 1
    
    return in_string


def remove_single_line_comments(content: str) -> str:
    """Remove single-line comments while preserving doc comments and strings."""
    lines = content.split('\n')
    result_lines = []
    in_block_comment = False
    file_header_done = False
    
    for line_num, line in enumerate(lines):
        # Track block comments
        if '/*' in line and '*/' not in line:
            in_block_comment = True
            result_lines.append(line)
            continue
        if in_block_comment:
            if '*/' in line:
                in_block_comment = False
            result_lines.append(line)
            continue
        
        # Preserve file-level block comments at the very top
        stripped = line.strip()
        if not file_header_done:
            if stripped.startswith('/*') or stripped.startswith('*') or stripped.startswith('*/'):
                result_lines.append(line)
                continue
            elif stripped == '' and line_num < 5:
                result_lines.append(line)
                continue
            else:
                file_header_done = True
        
        # Find // comments
        comment_match = re.search(r'//(?!/|!)', line)
        
        if comment_match:
            comment_pos = comment_match.start()
            
            # Check if it's inside a string
            if is_inside_string(line, comment_pos):
                result_lines.append(line)
                continue
            
            # Remove the comment part
            new_line = line[:comment_pos].rstrip()
            
            # If the line becomes empty or just whitespace, skip it entirely
            if new_line.strip() == '':
                # But preserve blank lines that were originally blank
                if line.strip().startswith('//'):
                    continue  # Skip comment-only lines
                else:
                    result_lines.append('')
            else:
                result_lines.append(new_line)
        else:
            result_lines.append(line)
    
    # Remove trailing empty lines but keep one
    while len(result_lines) > 1 and result_lines[-1] == '' and result_lines[-2] == '':
        result_lines.pop()
    
    return '\n'.join(result_lines)


def process_file(filepath: Path, dry_run: bool = False) -> tuple[bool, int]:
    """Process a single file. Returns (changed, lines_removed)."""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            original = f.read()
    except Exception as e:
        print(f"  Error reading {filepath}: {e}")
        return False, 0
    
    modified = remove_single_line_comments(original)
    
    if original == modified:
        return False, 0
    
    original_lines = len(original.split('\n'))
    modified_lines = len(modified.split('\n'))
    lines_removed = original_lines - modified_lines
    
    if dry_run:
        print(f"  Would modify: {filepath} (-{lines_removed} lines)")
    else:
        try:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(modified)
            print(f"  Modified: {filepath} (-{lines_removed} lines)")
        except Exception as e:
            print(f"  Error writing {filepath}: {e}")
            return False, 0
    
    return True, lines_removed


def main():
    parser = argparse.ArgumentParser(description='Remove single-line comments from Rust files')
    parser.add_argument('--dry-run', action='store_true', help='Show changes without modifying files')
    parser.add_argument('path', nargs='?', default='src', help='File or directory to process (default: src)')
    args = parser.parse_args()
    
    target = Path(args.path)
    
    if not target.exists():
        print(f"Error: {target} does not exist")
        sys.exit(1)
    
    if target.is_file():
        files = [target]
    else:
        files = list(target.rglob('*.rs'))
    
    print(f"Processing {len(files)} Rust files...")
    if args.dry_run:
        print("(DRY RUN - no files will be modified)\n")
    
    total_modified = 0
    total_lines_removed = 0
    
    for filepath in sorted(files):
        changed, lines = process_file(filepath, args.dry_run)
        if changed:
            total_modified += 1
            total_lines_removed += lines
    
    print(f"\nSummary:")
    print(f"  Files {'would be ' if args.dry_run else ''}modified: {total_modified}")
    print(f"  Lines {'would be ' if args.dry_run else ''}removed: {total_lines_removed}")
    
    if args.dry_run and total_modified > 0:
        print(f"\nRun without --dry-run to apply changes.")


if __name__ == '__main__':
    main()
