# Optional SELinux module for emobie-inputd

Fedora / RHEL users who see AVC denials after Grant can build and load this
module. Most installs work without it when session `setfacl` grants device access.
On typical Workstation installs the daemon runs as `unconfined_t`, so this module
is often a no-op — check denials first.

```bash
cd packaging/selinux
checkmodule -M -m -o emobie-inputd.mod emobie-inputd.te
semodule_package -o emobie-inputd.pp -m emobie-inputd.mod
pkexec --keep-cwd semodule -i emobie-inputd.pp
```

Inspect denials first:

```bash
ausearch -m avc -ts recent | grep -E 'emobie|input'
```

Remove the module:

```bash
pkexec --keep-cwd semodule -r emobie_inputd
```
