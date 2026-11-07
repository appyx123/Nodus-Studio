#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

BUILD_DIR="${ROOT_DIR}/build-rpm"
RPMTOP="${BUILD_DIR}/rpmbuild"

echo "=== Building Nodus Studio RPM ==="

rm -rf "${BUILD_DIR}"
mkdir -p "${RPMTOP}"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

VERSION="0.1.0"
TARBALL="${RPMTOP}/SOURCES/nodus-studio-${VERSION}.tar.gz"

echo "Creating source archive: ${TARBALL}..."
tar --exclude-vcs \
    --exclude='./build-rpm' \
    --exclude='./target' \
    --exclude='./src-tauri/target' \
    --exclude='./node_modules' \
    --transform "s,^\.,nodus-studio-${VERSION}," \
    -czf "${TARBALL}" -C "${ROOT_DIR}" .

echo "Running rpmbuild..."
rpmbuild -ba \
    --define "_topdir ${RPMTOP}" \
    "${ROOT_DIR}/packaging/rpm/nodus-studio.spec"

echo "=== Build Complete ==="
find "${RPMTOP}/RPMS" -type f -name "*.rpm"
find "${RPMTOP}/SRPMS" -type f -name "*.rpm"
