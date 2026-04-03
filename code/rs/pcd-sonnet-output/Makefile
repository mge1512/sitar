BINARY = sitar
PREFIX = /usr
BINDIR = $(PREFIX)/bin
MANDIR = $(PREFIX)/share/man/man1
DESTDIR =

.PHONY: build test install clean man

build:
	cargo build --release

test:
	cargo test

install: build man
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 target/release/$(BINARY) $(DESTDIR)$(BINDIR)/$(BINARY)
	install -d $(DESTDIR)$(MANDIR)
	install -m 644 $(BINARY).1 $(DESTDIR)$(MANDIR)/$(BINARY).1

clean:
	cargo clean
	rm -f $(BINARY).1

man: $(BINARY).1.md
	pandoc $(BINARY).1.md -s -t man -o $(BINARY).1
