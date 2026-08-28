# Optional SELinux module for emobie-inputd

Fedora / RHEL users who see AVC denials after Grant can build and load this
module. Most installs work without it when session `setfacl` grants device access.

```bash
cd packaging/selinux
checkmodule -M -m -o emobie-inputd.mod emobie-inputd.te
semodule_package -o emobie-inputd.pp -m emobie-inputd.mod
sudo semodule -i emobie-inputd.pp
```

Inspect denials first:

```bash
ausearch -m avc -ts recent | grep -E 'emobie|input'
```

Remove the module:

```bash
sudo semodule -r emobie_inputd
```
