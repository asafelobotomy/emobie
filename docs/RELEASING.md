# Releasing

## Update signing key (one-time setup)

The in-app updater installs packages as root (via `pkexec`) for deb/rpm
installs, so it does not trust a GitHub release on its own. Each release's
`SHA256SUMS` is signed in CI with a [minisign](https://jedisct1.github.io/minisign/)
key; the app embeds the public half (`src-tauri/update-signing.pub`) and only
offers one-click install when the signature verifies **and** its trusted
comment is `emobie <tag>` for that exact release.

Until the key is set up, the release workflow fails at "Sign checksums", and
builds containing the placeholder key never offer one-click install
(“Open release” still works).

1. Install minisign (`pacman -S minisign`, `apt install minisign`,
   `dnf install minisign`, or the release binary from GitHub).

2. Generate the key pair **on your own machine** (not in CI, not in a chat):

   ```bash
   mkdir -p ~/.minisign && chmod 700 ~/.minisign
   minisign -G -p src-tauri/update-signing.pub -s ~/.minisign/emobie-release.key
   ```

   It asks for a password. Use one: it protects the key file wherever it is
   copied. (`-W` creates an unencrypted key if you would rather not.)
   `-p` overwrites the placeholder in `src-tauri/update-signing.pub`, which is
   the file to commit.

3. Store the secret key and password as repository secrets:

   ```bash
   gh secret set MINISIGN_SECRET_KEY < ~/.minisign/emobie-release.key
   gh secret set MINISIGN_PASSWORD
   ```

   (`gh secret set MINISIGN_PASSWORD` prompts for the value; skip it for a
   `-W` key.)

4. Commit `src-tauri/update-signing.pub` and release as usual. The workflow
   signs `SHA256SUMS`, then checks the signature against the committed public
   key, so a mismatched secret fails the release instead of shipping
   unverifiable updates.

5. Back up `~/.minisign/emobie-release.key` and its password offline (a
   password manager or an encrypted USB stick).

### Lost or leaked key

The public key is compiled into every build, so updates to existing installs
can only be signed with the key those installs embed.

- **Rotating** (planned): copy the current key to
  `src-tauri/update-signing.rotate-from.pub`, generate the new key into
  `update-signing.pub`, and release once with the **old** secret still in
  `MINISIGN_SECRET_KEY` — installed copies accept that release, and it embeds
  the new key. Then delete `update-signing.rotate-from.pub`, set the secrets to
  the new key, and release normally from then on.
- **Lost key:** existing installs can no longer one-click update; publish the
  next release with a new key and tell users to install it manually once.
- **Leaked key:** delete the `MINISIGN_SECRET_KEY` secret, generate a new key,
  and ship a manual-install release as for a lost key. An attacker would also
  need to publish a GitHub release to use the leaked key.
