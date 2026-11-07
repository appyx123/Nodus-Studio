Name:           nodus-studio
Version:        0.1.0
Release:        1%{?dist}
Summary:        Desktop IoT and electronics simulation workbench

License:        MIT
URL:            https://github.com/appyx123/Nodus-Studio
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= 1.75
BuildRequires:  nodejs >= 18
BuildRequires:  npm
BuildRequires:  webkit2gtk4.1-devel
BuildRequires:  gtk3-devel
BuildRequires:  openssl-devel
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib

Requires:       webkit2gtk4.1
Requires:       gtk3
Requires:       openssl
Recommends:     arduino-cli

%description
Nodus Studio is an open-source interactive electronics workbench and IoT simulation studio.
It features an authoritative Rust simulation engine with cycle-accurate AVR emulation,
deterministic scheduling, digital logic analyzer, and virtual MQTT IoT integration.

%prep
%autosetup -p1

%build
# 1. Build SolidJS + TypeScript frontend bundle
npm run build

# 2. Build Rust desktop executable with offline vendor dependencies
cargo build --manifest-path src-tauri/Cargo.toml --release --locked

%install
# 1. Install desktop executable
install -Dm755 src-tauri/target/release/nodus %{buildroot}%{_bindir}/nodus

# 2. Install built-in component package registry
install -d %{buildroot}%{_datadir}/nodus-studio/components-registry
cp -r components-registry/* %{buildroot}%{_datadir}/nodus-studio/components-registry/

# 3. Install desktop entry file
install -Dm644 packaging/org.nodus.Studio.desktop %{buildroot}%{_datadir}/applications/org.nodus.Studio.desktop

# 4. Install application icon
install -Dm644 src-tauri/icons/icon.png %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/org.nodus.Studio.png

# 5. Install MIME type database entry
install -Dm644 packaging/org.nodus.Studio.xml %{buildroot}%{_datadir}/mime/packages/org.nodus.Studio.xml

# 6. Install AppStream metadata
install -Dm644 packaging/org.nodus.Studio.metainfo.xml %{buildroot}%{_metainfodir}/org.nodus.Studio.metainfo.xml

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/org.nodus.Studio.desktop
appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/org.nodus.Studio.metainfo.xml || true

%files
%license LICENSE
%doc README.md
%{_bindir}/nodus
%{_datadir}/nodus-studio/
%{_datadir}/applications/org.nodus.Studio.desktop
%{_datadir}/icons/hicolor/128x128/apps/org.nodus.Studio.png
%{_datadir}/mime/packages/org.nodus.Studio.xml
%{_metainfodir}/org.nodus.Studio.metainfo.xml

%changelog
* Sat Nov 07 2026 Nodus Studio Contributors <contributors@nodus.org> - 0.1.0-1
- Initial Fedora RPM release with AVR MCU emulation, logic analyzer, and .nodus project support
