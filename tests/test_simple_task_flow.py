from pathlib import Path


EXAMPLE_DIR = (
    Path(__file__).resolve().parents[1]
    / "examples"
    / "simple-task-flow"
)


def test_contract_worker_is_reference_runtime_example_without_external_services():
    worker_source = (EXAMPLE_DIR / "worker.py").read_text()

    forbidden_runtime_dependencies = [
        "NatsBus",
        "CingulaterClient",
        "localhost:8000",
        "asyncio.run(main())",
    ]

    for forbidden_dependency in forbidden_runtime_dependencies:
        assert forbidden_dependency not in worker_source


def test_contract_readme_documents_in_memory_reference_runtime_example():
    readme_source = (EXAMPLE_DIR / "README.md").read_text()
    readme_lower = readme_source.lower()

    assert "in-memory" in readme_lower
    assert "reference runtime example" in readme_lower
    assert "docker run" not in readme_lower
    assert "start nats" not in readme_lower
