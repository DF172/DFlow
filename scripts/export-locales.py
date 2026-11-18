#!/usr/bin/env python3
"""Export static desktop labels from the application's Fluent catalog."""

import argparse
import re
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
XML_LANG = '{http://www.w3.org/XML/1998/namespace}lang'
KEYS = {
    'comment': 'common.a-cad-application-for-2d-3d-drawing-and-design',
    'summary': 'common.value-2d-3d-cad-application-for-dwg-and-dxf-drawings',
    'description': 'common.create-and-edit-technical-drawings-in-dwg-and-dxf-formats',
    'drawing': 'common.drawing',
    'design': 'common.design',
    'engineering': 'common.engineering',
}


def labels(path):
    # Only static attributes are exported; the app resolves dynamic Fluent messages.
    catalog = {}
    group = key = None
    for line in path.read_text().splitlines():
        if match := re.match(r'^([A-Za-z][\w-]*)\s*=', line):
            group, key = match[1], None
        elif match := re.match(r'^    \.([A-Za-z][\w-]*)\s*=\s*(.*)', line):
            key = f'{group}.{match[1]}'
            catalog[key] = match[2]
        elif key and line.startswith('        '):
            catalog[key] += '\n' + line.strip()

    def resolve(key, seen=()):
        if key in seen:
            raise ValueError(f'{path}: cyclic reference {key}')
        text = re.sub(r'\{\s*([A-Za-z][\w-]*\.[A-Za-z][\w-]*)\s*\}',
                      lambda match: resolve(match[1], (*seen, key)), catalog[key])
        if re.search(r'[{}]|__df_', text):
            raise ValueError(f'{path}: {key} is not a static label')
        return text

    return {name: resolve(key) for name, key in KEYS.items()}


def outputs():
    localized = {path.parent.name: labels(path)
                 for path in sorted((ROOT / 'locales').glob('*/draftflow.ftl'))}

    desktop = ROOT / 'packaging/io.draftflow.DraftFlowCAD.desktop'
    lines = [line for line in desktop.read_text().splitlines()
             if not re.match(r'^(Comment|Keywords)(\[|=)', line)]
    for locale, text in localized.items():
        # Generic language fallbacks cover regional variants; Chinese has two scripts.
        suffix = '' if locale == 'en-US' else '[' + (
            locale.replace('-', '_') if locale.startswith('zh-') else locale.split('-')[0]) + ']'
        lines.append(f'Comment{suffix}={text["comment"]}')
        lines.append(f'Keywords{suffix}=CAD;DWG;DXF;{text["drawing"]};{text["design"]};{text["engineering"]};2D;3D;')
    yield desktop, '\n'.join(lines) + '\n'

    metainfo = ROOT / 'packaging/io.draftflow.DraftFlowCAD.metainfo.xml'
    component = ET.fromstring(metainfo.read_text())
    for child in list(component):
        if child.tag in ('summary', 'description', 'keywords'):
            component.remove(child)
    index = list(component).index(component.find('name')) + 1
    description = ET.Element('description')
    keywords = ET.Element('keywords')
    for word in ('CAD', 'DWG', 'DXF'):
        ET.SubElement(keywords, 'keyword').text = word
    for locale, text in localized.items():
        attributes = {} if locale == 'en-US' else {XML_LANG: locale}
        summary = ET.Element('summary', attributes)
        summary.text = text['summary']
        component.insert(index, summary)
        index += 1
        ET.SubElement(description, 'p', attributes).text = text['description']
        for key in ('drawing', 'design', 'engineering'):
            ET.SubElement(keywords, 'keyword', attributes).text = text[key]
    component.insert(index, description)
    component.insert(index + 1, keywords)
    ET.indent(component, space='  ')
    yield metainfo, '<?xml version="1.0" encoding="UTF-8"?>\n' + ET.tostring(component, encoding='unicode') + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='Fail if generated labels need updating.')
    args = parser.parse_args()
    stale = []
    for path, content in outputs():
        if path.exists() and path.read_text() == content:
            continue
        if args.check:
            stale.append(str(path.relative_to(ROOT)))
        else:
            path.write_text(content)
    if stale:
        parser.exit(1, 'Run python3 scripts/export-locales.py: ' + ', '.join(stale) + '\n')


if __name__ == '__main__':
    main()
