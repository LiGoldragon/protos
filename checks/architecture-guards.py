#!/usr/bin/env python3
"""Lexical architecture guards and their falsifiable fixture checks.

The production scan deliberately stays structural: it ignores comments and
string contents for Rust-scope checks, while the vocabulary guard examines the
raw source so that comments, documentation, and identifiers are all covered.
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Token:
    value: str
    line: int


def _skip_quoted(source: str, start: int, quote: str) -> tuple[int, int]:
    index = start + 1
    line = 0
    while index < len(source):
        character = source[index]
        if character == "\\":
            index += 2
            continue
        if character == quote:
            return index + 1, line
        if character == "\n":
            line += 1
        index += 1
    return index, line


def _looks_like_char_literal(source: str, start: int) -> bool:
    index = start + 1
    escaped = False
    while index < len(source) and source[index] != "\n":
        character = source[index]
        if escaped:
            escaped = False
        elif character == "\\":
            escaped = True
        elif character == "'":
            return index > start + 1
        index += 1
    return False


def _skip_raw_string(source: str, start: int) -> tuple[int, int] | None:
    marker = start
    if source[marker] == "b":
        marker += 1
    if marker >= len(source) or source[marker] != "r":
        return None
    marker += 1
    hashes = 0
    while marker < len(source) and source[marker] == "#":
        hashes += 1
        marker += 1
    if marker >= len(source) or source[marker] != '"':
        return None
    closing = '"' + ("#" * hashes)
    end = source.find(closing, marker + 1)
    if end < 0:
        return len(source), source.count("\n", marker)
    return end + len(closing), source.count("\n", start, end + len(closing))


def tokens(source: str) -> list[Token]:
    result: list[Token] = []
    index = 0
    line = 1
    while index < len(source):
        character = source[index]
        if character.isspace():
            if character == "\n":
                line += 1
            index += 1
            continue
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            if end < 0:
                break
            index = end
            continue
        if source.startswith("/*", index):
            depth = 1
            end = index + 2
            while end < len(source) and depth:
                if source.startswith("/*", end):
                    depth += 1
                    end += 2
                elif source.startswith("*/", end):
                    depth -= 1
                    end += 2
                else:
                    if source[end] == "\n":
                        line += 1
                    end += 1
            index = end
            continue
        raw_end = _skip_raw_string(source, index)
        if raw_end is not None:
            index, added_lines = raw_end
            line += added_lines
            continue
        if character == "'" and not _looks_like_char_literal(source, index):
            result.append(Token(character, line))
            index += 1
            continue
        if character in {'"', "'"}:
            index, added_lines = _skip_quoted(source, index, character)
            line += added_lines
            continue
        if character.isalpha() or character == "_":
            end = index + 1
            while end < len(source) and (source[end].isalnum() or source[end] == "_"):
                end += 1
            result.append(Token(source[index:end], line))
            index = end
            continue
        result.append(Token(character, line))
        index += 1
    return result


def matching(tokens_: list[Token]) -> dict[int, int]:
    pairs: dict[int, int] = {}
    opening = {"(": ")", "[": "]", "{": "}"}
    stack: list[tuple[str, int]] = []
    for index, token in enumerate(tokens_):
        if token.value in opening:
            stack.append((token.value, index))
        elif token.value in {"}", "]", ")"}:
            if not stack:
                continue
            open_value, open_index = stack.pop()
            if opening[open_value] == token.value:
                pairs[open_index] = index
                pairs[index] = open_index
    return pairs


def _is_function_pointer(tokens_: list[Token], index: int) -> bool:
    for token in reversed(tokens_[:index]):
        if token.value in {";", "{", "}"}:
            return False
        if token.value == "=":
            return True
    return False


def scope_violations(tokens_: list[Token]) -> list[str]:
    violations: list[str] = []
    scopes: list[str] = []
    pending: str | None = None
    for index, token in enumerate(tokens_):
        value = token.value
        if value in {"trait", "impl", "fn", "mod"}:
            pending = value
        elif value == "{":
            scopes.append(pending or "block")
            pending = None
        elif value == "}":
            if scopes:
                scopes.pop()
            pending = None
        elif value == ";":
            pending = None
        if value == "fn" and not _is_function_pointer(tokens_, index):
            nearest = next((scope for scope in reversed(scopes) if scope != "block"), None)
            if nearest not in {"trait", "impl"}:
                violations.append(f"line {token.line}: free function")
    return violations


def inherent_violations(tokens_: list[Token]) -> list[str]:
    violations: list[str] = []
    index = 0
    while index < len(tokens_):
        if tokens_[index].value != "impl":
            index += 1
            continue
        end = index + 1
        while end < len(tokens_) and tokens_[end].value not in {"{", ";"}:
            end += 1
        angle = paren = bracket = 0
        has_trait_for = False
        for token in tokens_[index + 1 : end]:
            if token.value == "where" and not (angle or paren or bracket):
                break
            if token.value == "for" and not (angle or paren or bracket):
                has_trait_for = True
            elif token.value == "<":
                angle += 1
            elif token.value == ">" and angle:
                angle -= 1
            elif token.value == "(":
                paren += 1
            elif token.value == ")" and paren:
                paren -= 1
            elif token.value == "[":
                bracket += 1
            elif token.value == "]" and bracket:
                bracket -= 1
        if end < len(tokens_) and tokens_[end].value == "{" and not has_trait_for:
            violations.append(f"line {tokens_[index].line}: inherent impl")
        index = end
    return violations


def _matching_angle(tokens_: list[Token], start: int) -> int | None:
    depth = 0
    for index in range(start, len(tokens_)):
        if tokens_[index].value == "<":
            depth += 1
        elif tokens_[index].value == ">" and depth:
            depth -= 1
            if depth == 0:
                return index
    return None


def zst_violations(tokens_: list[Token]) -> list[str]:
    pairs = matching(tokens_)
    zst_names: set[str] = set()
    index = 0
    while index < len(tokens_):
        if tokens_[index].value != "struct" or index + 1 >= len(tokens_):
            index += 1
            continue
        name = tokens_[index + 1].value
        if not (name[0].isalpha() or name[0] == "_"):
            index += 1
            continue
        marker = index + 2
        while marker < len(tokens_) and tokens_[marker].value not in {";", "(", "{"}:
            marker += 1
        empty = False
        if marker < len(tokens_) and tokens_[marker].value == ";":
            empty = True
        elif marker in pairs and tokens_[marker].value in {"(", "{"}:
            empty = pairs[marker] == marker + 1
        if empty:
            zst_names.add(name)
        index = marker + 1

    violations: list[str] = []
    index = 0
    while index < len(tokens_):
        if tokens_[index].value != "impl":
            index += 1
            continue
        end = index + 1
        while end < len(tokens_) and tokens_[end].value not in {"{", ";"}:
            end += 1
        if end < len(tokens_) and tokens_[end].value == "{" and zst_names:
            header = tokens_[index + 1 : end]
            for_index = next(
                (offset for offset, token in enumerate(header) if token.value == "for"),
                None,
            )
            target = header[for_index + 1 :] if for_index is not None else header
            if for_index is None and target and target[0].value == "<":
                generic_end = _matching_angle(tokens_, index + 1)
                if generic_end is not None:
                    target = tokens_[generic_end + 1 : end]
            target = target[: next(
                (offset for offset, token in enumerate(target) if token.value == "where"),
                len(target),
            )]
            path = []
            for token in target:
                if token.value in {"<", "["}:
                    break
                if token.value.isidentifier() and token.value not in {"dyn", "const", "unsafe"}:
                    path.append(token)
            if path and path[-1].value in zst_names:
                token = path[-1]
                violations.append(
                    f"line {token.line}: behavior attached to zero-sized {token.value}"
                )
        index = end
    return violations


FORBIDDEN = re.compile(
    r"(?<!\w)(archive|code|encode|decode|codec|transcode)(?!\w)",
    re.IGNORECASE,
)


def vocabulary_violations(source: str) -> list[str]:
    return [f"line {source.count(chr(10), 0, match.start()) + 1}: {match.group(0)}" for match in FORBIDDEN.finditer(source)]


GUARDS = {
    "free-functions": lambda source: scope_violations(tokens(source)),
    "inherent-methods": lambda source: inherent_violations(tokens(source)),
    "zst-behavior": lambda source: zst_violations(tokens(source)),
    "forbidden-vocabulary": vocabulary_violations,
}


def scan_source(guard: str, source: str) -> list[str]:
    return GUARDS[guard](source)


def scan_path(guard: str, path: Path) -> list[str]:
    return scan_source(guard, path.read_text(encoding="utf-8"))


def production_failures(source_root: Path, guard: str) -> list[str]:
    paths = sorted(source_root.rglob("*.rs"))
    if guard == "zst-behavior":
        source = "\n".join(path.read_text(encoding="utf-8") for path in paths)
        return [f"{source_root}: {failure}" for failure in scan_source(guard, source)]
    failures: list[str] = []
    for path in paths:
        failures.extend(f"{path}: {failure}" for failure in scan_path(guard, path))
    return failures


def fixture_failures(fixtures: Path, selected_guard: str | None = None) -> list[str]:
    failures: list[str] = []
    names = {
        "free-functions": ("free-functions", 6),
        "inherent-methods": ("inherent-methods", 1),
        "zst-behavior": ("zst", 4),
        "forbidden-vocabulary": ("vocabulary", 7),
    }
    if selected_guard is not None:
        names = {selected_guard: names[selected_guard]}
    for guard, (stem, minimum_bad_matches) in names.items():
        good = fixtures / f"{stem}-good.rs"
        if guard == "zst-behavior":
            bad_paths = [
                fixtures / "zst-bad.rs",
                fixtures / "zst-cross-file-decl.rs",
                fixtures / "zst-cross-file-impl.rs",
            ]
            bad_source = "\n".join(path.read_text(encoding="utf-8") for path in bad_paths)
            bad_failures = scan_source(guard, bad_source)
        else:
            bad = fixtures / f"{stem}-bad.rs"
            bad_failures = scan_path(guard, bad)
        good_failures = scan_path(guard, good)
        if len(bad_failures) < minimum_bad_matches:
            failures.append(
                f"{guard}: bad fixture reported {len(bad_failures)} matches; "
                f"expected at least {minimum_bad_matches}"
            )
        if good_failures:
            failures.append(f"{guard}: good fixture was rejected: {good_failures}")
    return failures


def main(arguments: list[str]) -> int:
    if len(arguments) not in {3, 5}:
        print(
            "usage: architecture-guards.py SOURCE_ROOT FIXTURE_ROOT [--guard NAME]",
            file=sys.stderr,
        )
        return 2
    source_root = Path(arguments[1])
    fixture_root = Path(arguments[2])
    if len(arguments) == 5:
        if arguments[3] != "--guard" or arguments[4] not in GUARDS:
            print("unknown guard", file=sys.stderr)
            return 2
        failures = fixture_failures(fixture_root, arguments[4])
        failures.extend(production_failures(source_root, arguments[4]))
    else:
        failures = fixture_failures(fixture_root)
        failures.extend(
            f"production/{guard}: {failure}"
            for guard in GUARDS
            for failure in production_failures(source_root, guard)
        )
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
