# Data Import & Export

`cdir` supports importing and exporting your data in YAML format, making it easy to:

- **Backup** your shortcuts and directory history
- **Share** shortcuts with teammates or across machines
- **Migrate** your data to a new computer

## Export Commands

### Export Shortcuts

Export all your shortcuts (bookmarks) to a YAML file:

```bash
$ cdir export-shortcuts ~/my-shortcuts.yaml
```

The exported file will contain:

```yaml
- name: projects
  path: /home/user/projects
  description: My projects folder
- name: docs
  path: /home/user/Documents
  description: null
```

### Export Paths

Export the current list of paths from your navigation history:

```bash
$ cdir export-current-paths ~/current-paths.yaml
```

This exports the most recent unique paths you've visited.

### Export Path History

Export the complete path history including multiple visits to the same directory:

```bash
$ cdir export-path-history ~/full-history.yaml
```

The exported file will contain:

```yaml
- date: '1740488400'
  path: /home/user/projects
- date: '1740488350'
  path: /home/user/Documents
- date: '1740488300'
  path: /home/user/projects
```

Each entry includes a UNIX timestamp (seconds since epoch).

## Import Commands

### Import Shortcuts

Import shortcuts from a YAML file:

```bash
$ cdir import-shortcuts ~/my-shortcuts.yaml
```

The file should contain shortcuts with `name` and `path` fields:

```yaml
- name: proj
  path: /home/user/projects
- name: tmp
  path: /tmp
  description: Temporary directory
```

### Import Paths

Import paths into both the current paths and history:

```bash
$ cdir import-paths ~/current-paths.yaml
```

This updates your current navigation state. The file format is:

```yaml
- date: '1740488400'
  path: /home/user/projects
- date: '1740488350'
  path: /home/user/Documents
```

### Import Path History

Import historical path data without affecting your current paths:

```bash
$ cdir import-path-history ~/full-history.yaml
```

**Important difference:** Unlike `import-paths`, this command only adds entries to the `paths_history` table and does NOT update your current paths. This is useful for:

- Merging historical data from multiple machines
- Restoring old navigation history
- Importing archived path data

## Common Use Cases

### Backup Everything

```bash
# Create a backup directory
mkdir ~/cdir-backup

# Export all your data
cdir export-shortcuts ~/cdir-backup/shortcuts.yaml
cdir export-current-paths ~/cdir-backup/current.yaml
cdir export-path-history ~/cdir-backup/history.yaml
```

### Setup a New Machine

```bash
# On the new machine, import your data
cdir import-shortcuts ~/cdir-backup/shortcuts.yaml
cdir import-path-history ~/cdir-backup/history.yaml
cdir import-paths ~/cdir-backup/current.yaml
```

### Share Shortcuts with Your Team

```bash
# Create a team shortcuts file
cat > team-shortcuts.yaml << EOF
- name: api
  path: /company/projects/api
  description: Main API service
- name: frontend
  path: /company/projects/frontend
  description: React frontend
- name: docs
  path: /company/docs
  description: Company documentation
EOF

# Everyone can import it
cdir import-shortcuts team-shortcuts.yaml
```
