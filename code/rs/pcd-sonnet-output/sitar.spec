Name:           sitar
Version:        0.9.0
Release:        1%{?dist}
Summary:        System InformaTion At Runtime
License:        GPL-2.0-or-later
URL:            https://build.opensuse.org/package/show/home:pcd/sitar
Source0:        sitar-0.9.0.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  pandoc
ExclusiveArch:  x86_64 aarch64

%description
sitar (System InformaTion At Runtime) collects hardware, kernel, network,
storage, security, and package information from a running Linux system
and renders it in one or more structured output formats, including a
machine-readable JSON format aligned with the Machinery system description
schema.

%prep
%autosetup

%build
pandoc sitar.1.md -s -t man -o sitar.1
cargo build --release

%install
install -d %{buildroot}%{_bindir}
install -m 755 target/release/sitar %{buildroot}%{_bindir}/sitar
install -d %{buildroot}%{_mandir}/man1
install -m 644 sitar.1 %{buildroot}%{_mandir}/man1/sitar.1
install -d %{buildroot}%{_sysconfdir}/sysconfig
cat > %{buildroot}%{_sysconfdir}/sysconfig/sitar << 'EOF'
## sitar configuration file
## See sitar(1) for details.
#SITAR_OPT_FORMAT=""
#SITAR_OPT_OUTDIR=""
#SITAR_OPT_OUTFILE=""
#SITAR_OPT_LIMIT="700000"
#SITAR_OPT_GCONF="false"
#SITAR_OPT_ALLCONFIGFILES="Auto"
#SITAR_OPT_ALLSUBDOMAIN="Auto"
#SITAR_OPT_ALLSYSCONFIG="Auto"
#SITAR_OPT_EXCLUDE="/etc/shadow"
#SITAR_OPT_LVMARCHIVE="false"
EOF
install -d %{buildroot}/var/lib/support

%files
%license LICENSE
%doc README.md
%{_bindir}/sitar
%{_mandir}/man1/sitar.1*
%config(noreplace) %{_sysconfdir}/sysconfig/sitar
%dir /var/lib/support

%changelog
* Fri Apr 03 2026 Matthias G. Eckermann <pcd@mailbox.org> - 0.9.0-1
- Initial release of sitar 0.9.0 in Rust
