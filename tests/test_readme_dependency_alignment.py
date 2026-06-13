from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[1]


def test_contract_readme_status_marks_runtime_llm_and_tool_as_minimal_reference_implemented():
    readme_source = (ROOT_DIR / "README.md").read_text()
    status_section = readme_source.split("## Status", 1)[1]
    implemented_section = status_section.split("Implemented today:", 1)[1].split(
        "Not implemented yet", 1
    )[0]
    not_implemented_section = status_section.split("Not implemented yet", 1)[1]

    runtime_subjects = [
        "runtime.llm.request",
        "runtime.llm.response",
        "runtime.tool.request",
        "runtime.tool.result",
    ]

    for runtime_subject in runtime_subjects:
        assert f"`{runtime_subject}`" in implemented_section
        assert runtime_subject not in not_implemented_section

    implemented_lower = implemented_section.lower()
    assert "minimal reference" in implemented_lower
    assert "implemented" in implemented_lower


def test_contract_pyproject_keeps_runtime_model_and_container_dependencies():
    pyproject_source = (ROOT_DIR / "pyproject.toml").read_text()

    assert '"pydantic>=2,<3"' in pyproject_source
    assert '"testcontainers==4.14.2"' in pyproject_source
