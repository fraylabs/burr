"""Download only the public, pinned URLs in sources.csv and verify STEP hashes.

CAD files remain local and ignored. Archives are cached beside local evidence;
only the explicit archive member is extracted. No login or generated CAD.
"""
import argparse
import csv
import hashlib
import pathlib
import tarfile
import urllib.request
import zipfile


def download(manifest, root):
    root.mkdir(parents=True,exist_ok=True)
    archives=root/'downloads'
    archives.mkdir(exist_ok=True)
    for row in csv.DictReader(manifest.open(newline='')):
        target=(root/row['file']).resolve()
        if not target.is_relative_to(root.resolve()):
            raise ValueError('Manifest destination escapes corpus directory')
        if target.exists() and hashlib.sha256(target.read_bytes()).hexdigest() == row['sha256']:
            continue
        member=row.get('archive_member')
        if member:
            archive=archives/hashlib.sha256(row['url'].encode()).hexdigest()
            if not archive.exists():
                with urllib.request.urlopen(row['url'],timeout=300) as response:
                    archive.write_bytes(response.read())
            if zipfile.is_zipfile(archive):
                with zipfile.ZipFile(archive) as source:
                    data=source.read(member)
            else:
                with tarfile.open(archive) as source:
                    data=source.extractfile(member).read()
        else:
            with urllib.request.urlopen(row['url'],timeout=300) as response:
                data=response.read()
        if hashlib.sha256(data).hexdigest() != row['sha256']:
            raise ValueError('Source hash mismatch: '+row['file'])
        target.parent.mkdir(parents=True,exist_ok=True)
        target.write_bytes(data)
        print(row['file'],len(data),flush=True)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('manifest',type=pathlib.Path)
    parser.add_argument('--root',type=pathlib.Path,required=True)
    args=parser.parse_args()
    download(args.manifest,args.root)
