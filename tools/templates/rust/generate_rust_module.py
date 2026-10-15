#!/usr/bin/env python3
"""
Genrl Rust Scaffolding Generator.
Generates:
  1. Folder modules (mod.rs, types.rs, logic.rs, db.rs, tests.rs)
  2. CLI command files with test stubs
  3. SQL migration files

Each generated file is strictly under 300 lines.
"""

import os
import sys
import argparse
import re

def to_snake_case(name: str) -> str:
    s = re.sub(r'(.)([A-Z][a-z]+)', r'\1_\2', name)
    return re.sub(r'([a-z0-9])([A-Z])', r'\1_\2', s).lower().strip('_')

def to_pascal_case(name: str) -> str:
    return ''.join(word.capitalize() for word in re.split(r'[-_]', name))

def generate_module(name: str, target_dir: str):
    module_name = to_snake_case(name)
    entity_name = to_pascal_case(name)
    dest_path = os.path.join(target_dir, module_name)
    os.makedirs(dest_path, exist_ok=True)

    template_dir = os.path.join(os.path.dirname(__file__), 'module')
    files = ['mod.rs', 'types.rs', 'logic.rs', 'db.rs', 'tests.rs']

    created = []
    for f in files:
        src = os.path.join(template_dir, f)
        dst = os.path.join(dest_path, f)
        with open(src, 'r', encoding='utf-8') as src_file:
            content = src_file.read()
        content = content.replace('{{MODULE_NAME}}', module_name)
        content = content.replace('{{ENTITY_NAME}}', entity_name)
        with open(dst, 'w', encoding='utf-8') as dst_file:
            dst_file.write(content)
        created.append(dst)

    print(f"[OK] Generated Rust module '{module_name}' at {dest_path}:")
    for c in created:
        print(f"  - {c}")

def generate_command(name: str, target_dir: str):
    cmd_name = to_snake_case(name)
    cmd_type = to_pascal_case(name)
    os.makedirs(target_dir, exist_ok=True)
    dst = os.path.join(target_dir, f"{cmd_name}.rs")

    template_path = os.path.join(os.path.dirname(__file__), 'cli_command', 'command.rs')
    with open(template_path, 'r', encoding='utf-8') as f:
        content = f.read()

    content = content.replace('{{COMMAND_NAME}}', cmd_name)
    content = content.replace('{{COMMAND_TYPE}}', cmd_type)

    with open(dst, 'w', encoding='utf-8') as f:
        f.write(content)

    print(f"[OK] Generated CLI command '{cmd_name}' at {dst}")

def generate_migration(name: str, target_dir: str):
    mig_name = to_snake_case(name)
    os.makedirs(target_dir, exist_ok=True)
    dst = os.path.join(target_dir, f"{mig_name}.sql")

    template_path = os.path.join(os.path.dirname(__file__), 'migration', 'migration.sql')
    with open(template_path, 'r', encoding='utf-8') as f:
        content = f.read()

    content = content.replace('{{MIGRATION_NAME}}', mig_name)
    content = content.replace('{{TABLE_NAME}}', mig_name)

    with open(dst, 'w', encoding='utf-8') as f:
        f.write(content)

    print(f"[OK] Generated SQL migration '{mig_name}' at {dst}")

def main():
    parser = argparse.ArgumentParser(description="Genrl Rust Module & Command Scaffolding")
    parser.add_argument('kind', choices=['module', 'command', 'migration'], help='Kind of template to generate')
    parser.add_argument('name', help='Name of the module, command, or table')
    parser.add_argument('--dest', default='.', help='Destination directory')

    args = parser.parse_args()
    if args.kind == 'module':
        generate_module(args.name, args.dest)
    elif args.kind == 'command':
        generate_command(args.name, args.dest)
    elif args.kind == 'migration':
        generate_migration(args.name, args.dest)

if __name__ == '__main__':
    main()
