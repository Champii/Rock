# Makefile for tree-sitter-rock

.PHONY: all generate test conformance build check clean

all: check

generate:
	tree-sitter generate

test: generate
	tree-sitter test

conformance: generate
	./scripts/check-repository-conformance.sh

build: generate
	tree-sitter build

check: test conformance build

clean:
	rm -rf build/
	rm -f rock.so parser.o
