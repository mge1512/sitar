Name:           sitar
Version:        0.9.0
Release:        0
License:        GPL-2.0-or-later
Summary:        System InformaTion At Runtime — collect and report system information
URL:            https://build.opensuse.org/package/show/home:mge/sitar
Source0:        %{name}-%{version}.tar.gz
BuildRequires:  golang >= 1.21
BuildRequires:  pandoc

%description
sitar (System InformaTion At Runtime) collects hardware, kernel, network,
storage, security, and package information from a running Linux system and
renders it in one or more structured output formats, including a
machine-readable JSON format aligned with the Machinery system description
schema.

Supported output formats: HTML, LaTeX, Simplified DocBook XML, JSON, Markdown.

%prep
%setup -q

%build
CGO_ENABLED=0 go build -ldflags="-s -w" -o %{name} .
pandoc %{name}.1.md -s -t man -o %{name}.1

%install
install -Dm755 %{name} %{buildroot}%{_bindir}/%{name}
install -Dm644 %{name}.1 %{buildroot}%{_mandir}/man1/%{name}.1

%files
%license LICENSE
%doc README.md
%{_bindir}/%{name}
%{_mandir}/man1/%{name}.1*

%changelog
