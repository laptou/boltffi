from __future__ import annotations

import re


METHOD_CASE_IDS: dict[str, str] = {
    "Noop": "noop",
    "EchoI32": "echo_i32",
    "Add": "add",
    "EchoStringSmall": "echo_string_small",
    "EchoString64K": "echo_string_64k",
    "GenerateString1K": "generate_string_1k",
    "GenerateString64K": "generate_string_64k",
    "MakePoint": "make_point",
    "EchoAddress": "echo_address",
    "EchoPerson": "echo_person",
    "EchoLine": "echo_line",
}


def method_name_to_case_id(method_name: str) -> str:
    known_case_id = METHOD_CASE_IDS.get(method_name)
    if known_case_id is not None:
        return known_case_id

    normalized_name = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", method_name).lower()
    normalized_name = normalized_name.replace("64_k", "64k").replace("1_k", "1k")
    return normalized_name
