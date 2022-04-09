#!/usr/bin/env python3
import os
import os.path
import subprocess

for path in os.listdir(path="seasons"):
    print(f"building index #{path}")
    name = os.path.splitext(path)[0]
    subprocess.run(["stork", "build", "--input", f"seasons/{path}", "--output", f"indexes/{name}.st"])
    