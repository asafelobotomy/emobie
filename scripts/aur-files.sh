#!/bin/sh
# Writes the AUR files for a release: aur/<pkgname>/ holds the PKGBUILD from
# DIR (packaging/aur/<pkgname> by default) with pkgver, pkgrel=1 and the
# checksum of GitHub's archive of TAG filled in, its .SRCINFO, and any other
# file in DIR (an .install script). Run as root in an Arch container (the
# release workflow's `aur` job); the files are pushed to the AUR by hand.
#
#   scripts/aur-files.sh v1.2.3 [DIR]
set -eu

tag=${1:?usage: scripts/aur-files.sh vX.Y.Z [DIR]}
dir=${2:-packaging/aur/emobie}
repo=${GITHUB_REPOSITORY:-asafelobotomy/emobie}
version=${tag#v}
case $version in
    *-*) echo "$tag is a pre-release: the AUR gets stable releases only"; exit 0 ;;
esac
case $version in
    *[!0-9.]* | '') echo "not a release tag: $tag" >&2; exit 1 ;;
esac

name=$(basename "$dir")
out=aur/$name
rm -rf "$out"
mkdir -p "$out"
cp -R "$dir"/. "$out"/

archive=$(mktemp)
trap 'rm -f "$archive"' EXIT
curl -fsSL --retry 3 -o "$archive" "https://github.com/$repo/archive/refs/tags/$tag.tar.gz"
sum=$(sha256sum "$archive" | cut -d' ' -f1)
sed -i -e "s/^pkgver=.*/pkgver=$version/" -e 's/^pkgrel=.*/pkgrel=1/' \
    -e "s/^sha256sums=.*/sha256sums=('$sum')/" "$out/PKGBUILD"

# makepkg refuses to run as root.
id -u builder >/dev/null 2>&1 || useradd -m builder
chown -R builder "$out"
su builder -c "cd '$PWD/$out' && makepkg --printsrcinfo > .SRCINFO"
grep -qx "	pkgver = $version" "$out/.SRCINFO"
grep -qx "	sha256sums = $sum" "$out/.SRCINFO"
echo "$out: $name $version, source sha256 $sum"
