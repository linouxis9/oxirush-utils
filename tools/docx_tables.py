#!/usr/bin/env python3
"""The paragraphs, tables and figures of a 3GPP specification in .docx, for the tests of
the OxiRush crates that compare their tables with the document's.

    import docx_tables
    body = docx_tables.document(open('29244-j60.zip', 'rb').read(), SHA256)
    for kind, block in docx_tables.blocks(body): ...

A crate's script takes this module from the oxirush-utils that it builds with (`cargo
metadata` gives its directory, `tools/` here), and keeps only what is of its document.
Run alone, it prints the paragraphs and the tables of a document, one a line:

    python3 tools/docx_tables.py FILE.zip|FILE.docx

Only the standard library is used.
"""
import hashlib
import io
import re
import sys
import urllib.request
import xml.etree.ElementTree as ET
import zipfile

W = '{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'

# The bit numbers of the header row of a figure, from the high one.
BITS = [str(bit) for bit in range(8, 0, -1)]


def fetch(url):
    """The octets at `url` of the 3GPP archive, which answers a browser's user agent."""
    request = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
    return urllib.request.urlopen(request, timeout=60).read()


def document(data, sha256=None):
    """The body of the document of `data`, an archive of the 3GPP archive that holds the
    .docx, or the .docx itself, checked against `sha256` where it is given."""
    if sha256 is not None:
        digest = hashlib.sha256(data).hexdigest()
        if digest != sha256:
            sys.exit(f'the document has SHA-256 {digest}, and {sha256} is expected')
    with zipfile.ZipFile(io.BytesIO(data)) as outer:
        names = outer.namelist()
        if 'word/document.xml' in names:
            return ET.fromstring(outer.read('word/document.xml')).find(W + 'body')
        docx = next(name for name in names if name.endswith('.docx'))
        with zipfile.ZipFile(io.BytesIO(outer.read(docx))) as inner:
            return ET.fromstring(inner.read('word/document.xml')).find(W + 'body')


def text(element):
    """The text of an element, its spaces made single."""
    out = []
    for node in element.iter():
        if node.tag == W + 't':
            out.append(node.text or '')
        elif node.tag in (W + 'tab', W + 'br', W + 'cr'):
            out.append(' ')
    return clean(''.join(out))


def clean(value):
    """A text with its spaces, quotes and apostrophes made plain and single."""
    value = value.replace(' ', ' ').replace('’', "'").replace('‘', "'")
    return re.sub(r'\s+', ' ', value).strip()


def blocks(body):
    """The paragraphs, as ('p', (style, text)), and the tables, as ('tbl', (rows, widths)):
    each row a list of (text, span), the paragraphs of a cell joined by newlines and its
    span in grid columns, and the widths of the cells of each row. A cell merged with the
    one above it (vMerge) has the text of that one."""
    for child in body:
        if child.tag == W + 'p':
            style = child.find(f'{W}pPr/{W}pStyle')
            value = text(child)
            if value:
                yield 'p', (style.get(W + 'val') if style is not None else '', value)
        elif child.tag == W + 'tbl':
            rows, widths = [], []
            above = {}
            for row in child.findall(W + 'tr'):
                cells, sizes = [], []
                before = row.find(f'{W}trPr/{W}gridBefore')
                column = int(before.get(W + 'val')) if before is not None else 0
                for cell in row.findall(W + 'tc'):
                    span = cell.find(f'{W}tcPr/{W}gridSpan')
                    width = cell.find(f'{W}tcPr/{W}tcW')
                    merged = cell.find(f'{W}tcPr/{W}vMerge')
                    # The paragraphs of a cell, one to a line: a name that the
                    # document writes on two lines is one name, which clean() reads.
                    paragraphs = [text(p) for p in cell.iter(W + 'p')]
                    value = '\n'.join(p for p in paragraphs if p)
                    if merged is not None and merged.get(W + 'val') != 'restart' and not value:
                        value = above.get(column, '')
                    above[column] = value
                    span = int(span.get(W + 'val')) if span is not None else 1
                    cells.append((value, span))
                    sizes.append(int(width.get(W + 'w')) if width is not None and width.get(W + 'type', 'dxa') == 'dxa' else None)
                    column += span
                rows.append(cells)
                widths.append(sizes)
            yield 'tbl', (rows, widths)
        elif child.tag == W + 'sdt':
            content = child.find(W + 'sdtContent')
            if content is not None:
                yield from blocks(content)


def without_notes(name):
    """A name without its references to notes: `(NOTE 1)`, `(see NOTE 2)`, `, see NOTE 3`."""
    name = re.sub(r'(?:,?\s*\((?:see |See )?NOTE[^)]*\))+', '', name)
    return clean(re.sub(r',\s*see NOTE \d+', '', name))


def places(rows, widths, header='Octets', label='left', by='width', skip=None):
    """The flags and fields of a figure, each [octets, name, lowest bit, number of bits],
    `octets` the label of its row.

    - `header`: the cell that the row of bit numbers has before them (`Octets`), or None
      for a figure whose row of bit numbers is the first of the cells "8" to "1".
    - `label`: where the label of the octets of a row is: 'left' (the first cell that has
      text, which the names follow), or 'right' (the last cell, after the names). A label
      on the right is given raw, its paragraphs joined by newlines, and an empty one
      continues the row above.
    - `by`: how the cells of a row are placed against the bit numbers: 'width', by the
      widths of the cells, the middle of each bit's cell within a field's; or 'grid', by
      the grid columns that the cells span, against those of the bit numbers.
    - `skip`: a regular expression of the names of cells that are no field.
    """
    if header is not None:
        at = next((at for at, row in enumerate(rows) if any(cell == header for cell, _ in row)), None)
    else:
        at = next((at for at, row in enumerate(rows) if [cell for cell, _ in row if cell][:8] == BITS), None)
    if at is None:
        return []
    if by == 'grid':
        return grid_places(rows[at:], label, skip)

    def edges(row, sizes, spans_of):
        start, out = 0, []
        for (cell, span), size in zip(row, sizes):
            width = size if size is not None else spans_of * span
            out.append((cell, start, start + width))
            start += width
        return out

    usable = all(size is not None for size in widths[at])
    middles = {}
    for cell, start, end in edges(rows[at], widths[at] if usable else [None] * len(rows[at]), 1):
        if re.fullmatch(r'[1-8]', cell):
            middles[int(cell)] = (start + end) / 2
    if len(middles) != 8:
        return []
    out = []
    for row, sizes in zip(rows[at + 1:], widths[at + 1:]):
        sizes = sizes if usable and all(size is not None for size in sizes) else [None] * len(row)
        cells = edges(row, sizes, 1)
        found = next(((cell, end) for cell, _, end in cells if cell and cell != '.'), None)
        if found is None or re.search(r'Type\s*=|Length\s*=', ' '.join(cell for cell, _, _ in cells)):
            continue
        octets, after = found
        for cell, start, end in cells:
            name = without_notes(cell)
            if start < after or not name or 'explicitly specified' in name or re.fullmatch(r'(?i)spare|\.\.\.|…|\d+', name):
                continue
            if skip is not None and re.search(skip, name):
                continue
            bits = sorted(bit for bit, middle in middles.items() if start <= middle < end)
            if bits:
                out.append([octets, name, str(bits[0]), str(len(bits))])
    return out


def grid_places(rows, label, skip):
    """`places` by the grid columns of the cells, from the row of bit numbers."""
    # The grid columns of each bit number, and of the label
    bit_columns, column = {}, 0
    for cell, span in rows[0]:
        if re.fullmatch(r'[1-8]', cell):
            bit_columns[int(cell)] = range(column, column + span)
        column += span
    if len(bit_columns) != 8:
        return []
    out, above = [], ''
    for row in rows[1:]:
        cells, column = [], 0
        for cell, span in row:
            cells.append((cell, column, column + span))
            column += span
        if label == 'right':
            octets = cells[-1][0] if cells else ''
            octets = octets or above
            above = octets
            named = cells[:-1]
        else:
            first = next((at for at, (cell, _, _) in enumerate(cells) if cell and cell != '.'), None)
            if first is None:
                continue
            octets, named = cells[first][0], cells[first + 1:]
        for cell, start, end in named:
            name = spare(without_notes(cell))
            if not name or 'explicitly specified' in name:
                continue
            if skip is not None and re.search(skip, name):
                continue
            bits = sorted(bit for bit, columns in bit_columns.items() if any(start <= c < end for c in columns))
            if bits:
                out.append([octets, name, str(bits[0]), str(len(bits))])
    return out


def spare(name):
    """A spare cell by one name, `Spare`, whether the document writes its value of 0 above
    it or not."""
    return 'Spare' if re.fullmatch(r'(?i)(?:0 )?spare', name) else name


def offsets(label):
    """The octets of a row of a figure that its label counts from a letter: "m" is (m, 0,
    0), "(m+1) to (m+2)" (m, 1, 2), "a to (a+1)" (a, 0, 1); None for another label."""
    label = re.sub(r'[\s"\']', '', label)
    found = re.fullmatch(r'\(?([a-z])(?:\+(\d+))?\)?(?:to\(?\1(?:\+(\d+))?\)?)?', label)
    if not found:
        return None
    low = int(found.group(2) or 0)
    return found.group(1), low, int(found.group(3) or low)


def octet(one):
    """One octet of a label: ('', 3) for "3", ('i', 1) for "i+1", ('k', 1) for "(k+1)",
    with whether it is optional (a `*` after it); None for another."""
    one = re.sub(r'\s', '', one)
    found = re.fullmatch(r'\(?(?:(\d+)|([a-z])(?:\+(\d+))?)\)?(\*)?', one)
    if not found:
        return None
    number, letter, plus, star = found.groups()
    if number is not None:
        return ('', int(number)), bool(star)
    return (letter, int(plus or 0)), bool(star)


def octet_label(label):
    """The first and last octets of the raw label of a row of a figure of the NAS
    specifications, each (letter or '', offset), and whether the octets are optional:
    "octet 3", "octet 3\\noctet i" (a cell that spans rows), "octet 4*", "octet i+1*",
    "octet (k+1)*", "octet m+1*\\noctet n*", "octet 5-6", "octets 5-n", "Octet 3",
    "octet 23*- 25*", "octet 3\\nto". None for another label."""
    words = re.sub(r'(?i)\boctets?\b', ' ', label.replace('\n', ' '))
    words = re.sub(r'\bto\b', ' - ', words).strip()
    parts = [part for part in re.split(r'\s*-\s*|\s{2,}|\s+(?=\(?[0-9a-z])', words) if part]
    if not parts:
        return None
    read = [octet(part) for part in parts]
    if any(found is None for found in read):
        return None
    first, last = read[0], read[-1]
    return first[0], last[0], any(optional for _, optional in read)


def main():
    data = open(sys.argv[1], 'rb').read()
    for kind, block in blocks(document(data)):
        if kind == 'p':
            style, value = block
            print(f'[{style}] {value}')
        else:
            rows, _ = block
            for row in rows:
                print('| ' + ' | '.join(f'{cell}' + (f' <{span}>' if span > 1 else '') for cell, span in row).replace('\n', '\\n') + ' |')


if __name__ == '__main__':
    main()
