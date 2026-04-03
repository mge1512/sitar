BINARY_NAME := sitar
VERSION     := 0.9.0
PREFIX      := /usr
BINDIR      := $(PREFIX)/bin
MANDIR      := $(PREFIX)/share/man/man1

.PHONY: build test install clean man

build:
	CGO_ENABLED=0 go build -ldflags="-s -w" -o $(BINARY_NAME) .

test:
	go test ./...

install: build man
	install -Dm755 $(BINARY_NAME) $(DESTDIR)$(BINDIR)/$(BINARY_NAME)
	install -Dm644 $(BINARY_NAME).1 $(DESTDIR)$(MANDIR)/$(BINARY_NAME).1

clean:
	rm -f $(BINARY_NAME) $(BINARY_NAME).1

man:
	pandoc $(BINARY_NAME).1.md -s -t man -o $(BINARY_NAME).1
