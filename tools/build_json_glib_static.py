#!/usr/bin/env python3
import os
import subprocess
import urllib.request
import tarfile
import lzma
import io

DEST_A = '/usr/lib/arm-linux-gnueabi/libjson-glib-1.0.a'
if os.path.exists(DEST_A):
    print(f"{DEST_A} already exists, skipping build.")
    exit(0)

print("Building static libjson-glib-1.0.a for Kindle OS target...")
url = 'http://archive.debian.org/debian/pool/main/j/json-glib/json-glib_1.4.4.orig.tar.xz'
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
with urllib.request.urlopen(req) as resp:
    data = resp.read()

tar_bytes = lzma.decompress(data)
build_dir = '/tmp/json-glib-build'
os.makedirs(build_dir, exist_ok=True)
with tarfile.open(fileobj=io.BytesIO(tar_bytes)) as tar:
    tar.extractall(build_dir)

src_dir = os.path.join(build_dir, 'json-glib-1.4.4', 'json-glib')
with open(os.path.join(src_dir, 'config.h'), 'w') as f:
    f.write('#pragma once\n#define HAVE_UNISTD_H 1\n#define GETTEXT_PACKAGE "json-glib-1.0"\n')

enum_c = """#ifndef JSON_COMPILATION
#define JSON_COMPILATION
#endif
#include "config.h"
#include <json-glib/json-glib.h>
#include <json-glib/json-enum-types.h>

GType json_parser_error_get_type (void) {
  static volatile gsize id = 0;
  if (g_once_init_enter (&id)) {
    static const GEnumValue values[] = {
      { JSON_PARSER_ERROR_PARSE, "JSON_PARSER_ERROR_PARSE", "parse" },
      { JSON_PARSER_ERROR_TRAILING_COMMA, "JSON_PARSER_ERROR_TRAILING_COMMA", "trailing-comma" },
      { JSON_PARSER_ERROR_MISSING_COMMA, "JSON_PARSER_ERROR_MISSING_COMMA", "missing-comma" },
      { JSON_PARSER_ERROR_MISSING_COLON, "JSON_PARSER_ERROR_MISSING_COLON", "missing-colon" },
      { JSON_PARSER_ERROR_INVALID_BAREWORD, "JSON_PARSER_ERROR_INVALID_BAREWORD", "invalid-bareword" },
      { JSON_PARSER_ERROR_EMPTY_MEMBER_NAME, "JSON_PARSER_ERROR_EMPTY_MEMBER_NAME", "empty-member-name" },
      { JSON_PARSER_ERROR_INVALID_DATA, "JSON_PARSER_ERROR_INVALID_DATA", "invalid-data" },
      { JSON_PARSER_ERROR_UNKNOWN, "JSON_PARSER_ERROR_UNKNOWN", "unknown" },
      { 0, NULL, NULL }
    };
    GType g_id = g_enum_register_static (g_intern_static_string ("JsonParserError"), values);
    g_once_init_leave (&id, g_id);
  }
  return id;
}

GType json_path_error_get_type (void) {
  static volatile gsize id = 0;
  if (g_once_init_enter (&id)) {
    static const GEnumValue values[] = {
      { JSON_PATH_ERROR_INVALID_QUERY, "JSON_PATH_ERROR_INVALID_QUERY", "invalid-query" },
      { 0, NULL, NULL }
    };
    GType g_id = g_enum_register_static (g_intern_static_string ("JsonPathError"), values);
    g_once_init_leave (&id, g_id);
  }
  return id;
}

GType json_reader_error_get_type (void) {
  static volatile gsize id = 0;
  if (g_once_init_enter (&id)) {
    static const GEnumValue values[] = {
      { JSON_READER_ERROR_NO_ARRAY, "JSON_READER_ERROR_NO_ARRAY", "no-array" },
      { JSON_READER_ERROR_INVALID_INDEX, "JSON_READER_ERROR_INVALID_INDEX", "invalid-index" },
      { JSON_READER_ERROR_NO_OBJECT, "JSON_READER_ERROR_NO_OBJECT", "no-object" },
      { JSON_READER_ERROR_INVALID_MEMBER, "JSON_READER_ERROR_INVALID_MEMBER", "invalid-member" },
      { JSON_READER_ERROR_INVALID_NODE, "JSON_READER_ERROR_INVALID_NODE", "invalid-node" },
      { 0, NULL, NULL }
    };
    GType g_id = g_enum_register_static (g_intern_static_string ("JsonReaderError"), values);
    g_once_init_leave (&id, g_id);
  }
  return id;
}

GType json_node_type_get_type (void) {
  static volatile gsize id = 0;
  if (g_once_init_enter (&id)) {
    static const GEnumValue values[] = {
      { JSON_NODE_OBJECT, "JSON_NODE_OBJECT", "object" },
      { JSON_NODE_ARRAY, "JSON_NODE_ARRAY", "array" },
      { JSON_NODE_VALUE, "JSON_NODE_VALUE", "value" },
      { JSON_NODE_NULL, "JSON_NODE_NULL", "null" },
      { 0, NULL, NULL }
    };
    GType g_id = g_enum_register_static (g_intern_static_string ("JsonNodeType"), values);
    g_once_init_leave (&id, g_id);
  }
  return id;
}
"""
with open(os.path.join(src_dir, 'json-enum-types.c'), 'w') as f:
    f.write(enum_c)

c_files = [
    'json-array.c', 'json-builder.c', 'json-debug.c', 'json-enum-types.c',
    'json-gboxed.c', 'json-generator.c', 'json-gobject.c', 'json-gvariant.c',
    'json-node.c', 'json-object.c', 'json-parser.c', 'json-path.c',
    'json-reader.c', 'json-scanner.c', 'json-serializable.c', 'json-utils.c',
    'json-value.c'
]
o_files = []
for f in c_files:
    o = f.replace('.c', '.o')
    o_files.append(o)
    cmd = [
        'arm-linux-gnueabi-gcc', '-O2', '-fPIC', '-c',
        '-I.', '-I..',
        '-I/usr/include/glib-2.0',
        '-I/usr/lib/arm-linux-gnueabi/glib-2.0/include',
        '-I/usr/include/json-glib-1.0',
        '-DJSON_COMPILATION',
        f, '-o', o
    ]
    subprocess.check_call(cmd, cwd=src_dir)

subprocess.check_call(['arm-linux-gnueabi-ar', 'rcs', DEST_A] + o_files, cwd=src_dir)
print(f"Successfully built {DEST_A} ({os.path.getsize(DEST_A)} bytes)")
