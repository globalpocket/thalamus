#!/usr/bin/env python3
import re
import sys
from pathlib import Path


TARGET_SUFFIX = "rust/protocol/src/message.rs"
DECLARATION_EXCLUDE_LINES = {
    17,
    18,
    19,
    20,
    21,
    22,
    23,
    29,
    30,
    31,
    32,
    33,
    34,
    35,
    36,
    37,
    38,
}


def is_target_source(source_line):
    return source_line.startswith("SF:") and source_line[3:].endswith(TARGET_SUFFIX)


def line_number(line, prefix):
    match = re.match(rf"{prefix}:(\d+),", line)
    if match is None:
        return None
    return int(match.group(1))


def function_name(line, prefix):
    parts = line.split(",", 1)
    if len(parts) != 2 or not parts[0].startswith(f"{prefix}:"):
        return None
    return parts[1]


def normalize_target_record(record):
    excluded_functions = {
        function_name(line, "FN")
        for line in record
        if (line_number(line, "FN") in DECLARATION_EXCLUDE_LINES)
    }
    excluded_functions.discard(None)

    normalized = []
    remaining_da_hits = []
    remaining_function_names = set()

    for line in record:
        da_line = line_number(line, "DA")
        fn_line = line_number(line, "FN")
        fnda_name = function_name(line, "FNDA")

        if da_line in DECLARATION_EXCLUDE_LINES:
            continue
        if fn_line in DECLARATION_EXCLUDE_LINES:
            continue
        if fnda_name in excluded_functions:
            continue
        if line.startswith(("LF:", "LH:", "FNF:", "FNH:")):
            continue

        normalized.append(line)

        if da_line is not None:
            hits_text = line.split(",", 2)[1]
            remaining_da_hits.append(int(hits_text))
        if fn_line is not None:
            fn_name = function_name(line, "FN")
            if fn_name is not None:
                remaining_function_names.add(fn_name)

    function_hits = {
        function_name(line, "FNDA")
        for line in normalized
        if line.startswith("FNDA:") and not line.startswith("FNDA:0,")
    }
    function_hits.discard(None)

    insert_at = next(
        (index for index, line in enumerate(normalized) if line.startswith(("DA:", "BRF:", "BRH:", "end_of_record"))),
        len(normalized),
    )
    normalized[insert_at:insert_at] = [
        f"FNF:{len(remaining_function_names)}",
        f"FNH:{len(remaining_function_names & function_hits)}",
    ]

    end_index = normalized.index("end_of_record") if "end_of_record" in normalized else len(normalized)
    normalized[end_index:end_index] = [
        f"LF:{len(remaining_da_hits)}",
        f"LH:{sum(1 for hits in remaining_da_hits if hits > 0)}",
    ]
    return normalized


def split_records(lines):
    records = []
    record = []
    for line in lines:
        record.append(line)
        if line == "end_of_record":
            records.append(record)
            record = []
    if record:
        records.append(record)
    return records


def normalize_lcov(input_path, output_path):
    input_text = input_path.read_text(encoding="utf-8")
    lines = input_text.splitlines()
    output_lines = []

    for record in split_records(lines):
        if record and is_target_source(record[0]):
            output_lines.extend(normalize_target_record(record))
        else:
            output_lines.extend(record)

    output_path.write_text("\n".join(output_lines) + "\n", encoding="utf-8")


def main(argv):
    if len(argv) != 3:
        print("usage: normalize-rust-protocol-lcov.py <input.info> <output.info>", file=sys.stderr)
        return 2
    normalize_lcov(Path(argv[1]), Path(argv[2]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
