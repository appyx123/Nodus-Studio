#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

VERSION="${1:-0.1.0}"
ARCH="linux-x86_64"
DIST_NAME="nodus-studio-${VERSION}-${ARCH}"
DIST_DIR="${ROOT_DIR}/dist-release/${DIST_NAME}"
OUTPUT_TARBALL="${ROOT_DIR}/dist-release/${DIST_NAME}.tar.gz"

echo "=== Packaging Nodus Studio Portable Release: ${DIST_NAME} ==="

# 1. Clean previous packaging directory
rm -rf "${ROOT_DIR}/dist-release"
mkdir -p "${DIST_DIR}"/bin
mkdir -p "${DIST_DIR}"/share/nodus-studio
mkdir -p "${DIST_DIR}"/share/applications
mkdir -p "${DIST_DIR}"/share/icons/hicolor/128x128/apps
mkdir -p "${DIST_DIR}"/share/icons/hicolor/512x512/apps
mkdir -p "${DIST_DIR}"/share/metainfo
mkdir -p "${DIST_DIR}"/share/mime/packages

# 2. Check and copy release binary
BINARY_PATH="${ROOT_DIR}/src-tauri/target/release/nodus"
if [[ ! -f "${BINARY_PATH}" ]]; then
  echo "Error: Binary not found at ${BINARY_PATH}."
  echo "Please build with: npm run tauri -- build --no-bundle"
  exit 1
fi
cp "${BINARY_PATH}" "${DIST_DIR}/bin/nodus"
chmod 755 "${DIST_DIR}/bin/nodus"

# 3. Copy component packages registry
cp -r "${ROOT_DIR}/components-registry" "${DIST_DIR}/share/nodus-studio/"
cp -r "${ROOT_DIR}/components-registry" "${DIST_DIR}/components-registry"

# 4. Copy desktop integration assets
cp "${ROOT_DIR}/packaging/org.nodus.Studio.desktop" "${DIST_DIR}/share/applications/"
cp "${ROOT_DIR}/src-tauri/icons/128x128.png" "${DIST_DIR}/share/icons/hicolor/128x128/apps/org.nodus.Studio.png"
cp "${ROOT_DIR}/src-tauri/icons/icon.png" "${DIST_DIR}/share/icons/hicolor/512x512/apps/org.nodus.Studio.png"
cp "${ROOT_DIR}/packaging/org.nodus.Studio.metainfo.xml" "${DIST_DIR}/share/metainfo/"
cp "${ROOT_DIR}/packaging/org.nodus.Studio.xml" "${DIST_DIR}/share/mime/packages/"

# 5. Copy license and docs
cp "${ROOT_DIR}/LICENSE" "${DIST_DIR}/"
cp "${ROOT_DIR}/README.md" "${DIST_DIR}/"
if [[ -d "${ROOT_DIR}/examples" ]]; then
  cp -r "${ROOT_DIR}/examples" "${DIST_DIR}/"
fi

# 6. Create universal user-friendly installer script
cat <<'EOF' > "${DIST_DIR}/install.sh"
#!/usr/bin/env bash
set -euo pipefail

PREFIX="${1:-${HOME}/.local}"
echo "Installing Nodus Studio into: ${PREFIX}"

mkdir -p "${PREFIX}/bin"
mkdir -p "${PREFIX}/share/nodus-studio/components-registry"
mkdir -p "${PREFIX}/share/applications"
mkdir -p "${PREFIX}/share/icons/hicolor/128x128/apps"
mkdir -p "${PREFIX}/share/icons/hicolor/512x512/apps"
mkdir -p "${PREFIX}/share/metainfo"
mkdir -p "${PREFIX}/share/mime/packages"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

install -m 755 "${SCRIPT_DIR}/bin/nodus" "${PREFIX}/bin/nodus"
cp -r "${SCRIPT_DIR}/share/nodus-studio/components-registry/"* "${PREFIX}/share/nodus-studio/components-registry/"
install -m 644 "${SCRIPT_DIR}/share/applications/org.nodus.Studio.desktop" "${PREFIX}/share/applications/"
install -m 644 "${SCRIPT_DIR}/share/icons/hicolor/128x128/apps/org.nodus.Studio.png" "${PREFIX}/share/icons/hicolor/128x128/apps/"
install -m 644 "${SCRIPT_DIR}/share/icons/hicolor/512x512/apps/org.nodus.Studio.png" "${PREFIX}/share/icons/hicolor/512x512/apps/"
install -m 644 "${SCRIPT_DIR}/share/metainfo/org.nodus.Studio.metainfo.xml" "${PREFIX}/share/metainfo/"
install -m 644 "${SCRIPT_DIR}/share/mime/packages/org.nodus.Studio.xml" "${PREFIX}/share/mime/packages/"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${PREFIX}/share/applications" || true
fi
if command -v update-mime-database >/dev/null 2>&1; then
  update-mime-database "${PREFIX}/share/mime" || true
fi

echo "Nodus Studio installed successfully!"
echo "You can launch it by running: ${PREFIX}/bin/nodus"
EOF
chmod +x "${DIST_DIR}/install.sh"

# 7. Create uninstall script
cat <<'EOF' > "${DIST_DIR}/uninstall.sh"
#!/usr/bin/env bash
set -euo pipefail

PREFIX="${1:-${HOME}/.local}"
echo "Uninstalling Nodus Studio from: ${PREFIX}"

rm -f "${PREFIX}/bin/nodus"
rm -rf "${PREFIX}/share/nodus-studio"
rm -f "${PREFIX}/share/applications/org.nodus.Studio.desktop"
rm -f "${PREFIX}/share/icons/hicolor/128x128/apps/org.nodus.Studio.png"
rm -f "${PREFIX}/share/icons/hicolor/512x512/apps/org.nodus.Studio.png"
rm -f "${PREFIX}/share/metainfo/org.nodus.Studio.metainfo.xml"
rm -f "${PREFIX}/share/mime/packages/org.nodus.Studio.xml"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${PREFIX}/share/applications" || true
fi
if command -v update-mime-database >/dev/null 2>&1; then
  update-mime-database "${PREFIX}/share/mime" || true
fi

echo "Nodus Studio uninstalled."
EOF
chmod +x "${DIST_DIR}/uninstall.sh"

# 8. Create compressed tarball
tar -czf "${OUTPUT_TARBALL}" -C "${ROOT_DIR}/dist-release" "${DIST_NAME}"
(cd "${ROOT_DIR}/dist-release" && sha256sum "${DIST_NAME}.tar.gz" > "${DIST_NAME}.tar.gz.sha256")

echo "=== Release package created successfully ==="
echo "Tarball: ${OUTPUT_TARBALL}"
echo "Checksum: $(cat "${OUTPUT_TARBALL}.sha256")"
