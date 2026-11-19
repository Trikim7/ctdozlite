"""Bounded, read-only text/table preview. Never execute document macros."""
import json
import itertools
import select
import subprocess
import sys
import time
import zipfile
import xml.etree.ElementTree as ET

try:
    import resource
    resource.setrlimit(resource.RLIMIT_AS, (384 * 1024**2, 384 * 1024**2))
    resource.setrlimit(resource.RLIMIT_CPU, (12, 12))
except (ImportError, ValueError, OSError):
    pass

MAX_TEXT = 24000
MAX_XML = 12 * 1024**2

def xml_member(z, name):
    info = z.getinfo(name)
    if info.file_size > MAX_XML:
        raise ValueError("Tài liệu nén quá lớn")
    with z.open(info) as f:
        raw = f.read(MAX_XML + 1)
    if len(raw) > MAX_XML or b"\0" in raw or b"<!DOCTYPE" in raw or b"<!ENTITY" in raw:
        raise ValueError("XML không phù hợp để xem trước")
    return ET.fromstring(raw)

def tag(element):
    return element.tag.rsplit("}", 1)[-1]

def docx(path):
    with zipfile.ZipFile(path) as z:
        root = xml_member(z, "word/document.xml")
        return "\n".join("".join(e.text or "" for e in p.iter() if tag(e) == "t")
                         for p in root.iter() if tag(p) == "p")

def xlsx(path):
    blocks = []
    with zipfile.ZipFile(path) as z:
        shared = []
        if "xl/sharedStrings.xml" in z.namelist():
            shared = ["".join(t.text or "" for t in si.iter() if tag(t) == "t")
                      for si in xml_member(z, "xl/sharedStrings.xml") if tag(si) == "si"]
        names = {}
        workbook = xml_member(z, "xl/workbook.xml")
        rels = xml_member(z, "xl/_rels/workbook.xml.rels")
        targets = {e.get("Id"):e.get("Target","") for e in rels if e.get("TargetMode") != "External"}
        for sheet in workbook.iter():
            if tag(sheet) != "sheet":
                continue
            rid = next((v for k,v in sheet.attrib.items() if k.endswith("}id")), "")
            target = targets.get(rid, "")
            target = target.lstrip("/") if target.startswith("/") else "xl/" + target
            if target.startswith("xl/worksheets/") and ".." not in target.split("/"):
                names[target] = sheet.get("name", "Sheet")
        for path_name, name in list(names.items())[:8]:
            if path_name not in z.namelist():
                continue
            root = xml_member(z, path_name)
            blocks.append("=== " + name + " ===")
            for row in itertools.islice((r for r in root.iter() if tag(r) == "row"),80):
                cells = []
                for cell in itertools.islice((c for c in row if tag(c) == "c"),24):
                    col = 0
                    for char in cell.get("r",""):
                        if not ("A" <= char <= "Z"):
                            break
                        col = col * 26 + ord(char) - ord("A") + 1
                    if col > 24:
                        continue
                    while col and len(cells) < col - 1:
                        cells.append("")
                    value = next((e.text or "" for e in cell if tag(e) == "v"), "")
                    if cell.get("t") == "s":
                        try: value = shared[int(value)]
                        except (ValueError, IndexError): value = ""
                    elif cell.get("t") == "inlineStr":
                        value = "".join(e.text or "" for e in cell.iter() if tag(e) == "t")
                    cells.append(value.replace("\t"," ").replace("\n"," ")[:240])
                blocks.append("\t".join(cells))
                if sum(map(len,blocks)) > MAX_TEXT:
                    return "\n".join(blocks)
    return "\n".join(blocks)

def external(args):
    # The helper process inherits memory/CPU limits. Bound output through a pipe.
    with subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL) as p:
        try:
            raw = bytearray()
            deadline = time.monotonic() + 12
            while len(raw) < 96 * 1024:
                remaining = deadline - time.monotonic()
                if remaining <= 0 or not select.select([p.stdout], [], [], remaining)[0]:
                    raise ValueError("Hết thời gian đọc")
                chunk = p.stdout.read1(min(8192,96*1024-len(raw)))
                if not chunk:
                    break
                raw.extend(chunk)
            if len(raw) >= 96 * 1024:
                p.kill()
            else:
                if p.wait(timeout=12) != 0:
                    raise ValueError("Không đọc được nội dung tài liệu")
            return raw.decode("utf-8", errors="replace")
        finally:
            if p.poll() is None: p.kill()

def main():
    path, ext = sys.argv[1:]
    if ext == "pdf":
        text = external(["pdftotext", "-f", "1", "-l", "10", "-layout", path, "-"])
    elif ext == "docx":
        text = docx(path)
    elif ext in ("xlsx", "xlsm"):
        text = xlsx(path)
    elif ext == "doc":
        try:
            text = external(["catdoc", "-d", "utf-8", path])
        except FileNotFoundError:
            text = external(["antiword", "-m", "UTF-8.txt", path])
    elif ext == "xls":
        text = external(["xls2csv", "-d", "utf-8", path])
    else:
        with open(path,"rb") as f:
            raw = f.read(96 * 1024)
        text = raw.decode("utf-16" if raw.startswith((b"\xff\xfe",b"\xfe\xff")) else "utf-8-sig",errors="replace")
    if not text.strip():
        text = "Không có văn bản để hiển thị. PDF scan hoặc tài liệu chứa ảnh cần mở bằng ứng dụng chuyên dụng."
    print(json.dumps({"text":text[:MAX_TEXT],"truncated":len(text)>MAX_TEXT or ext in ("pdf","xlsx","xlsm")},ensure_ascii=False))

if __name__ == "__main__":
    try:
        main()
    except FileNotFoundError:
        print(json.dumps({"error":"Thiếu bộ đọc định dạng này trên máy"}))
    except Exception:
        print(json.dumps({"error":"Không đọc được nội dung hoặc tài liệu vượt giới hạn xem trước"}))
