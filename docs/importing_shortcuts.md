# Importing Shortcuts

cdir supports importing shortcuts from a YAML file.

To do so, the file should contain the list of shortcuts defined by a `name` and a `path`.

Example:

```yaml
- name: t1
  path: /tmp1
- name: t2
  path: /tmp2
  description: Temporary directory 2
```

Import with:

```
$ cdir import-shortcuts /path/to/shortcuts.yaml
```