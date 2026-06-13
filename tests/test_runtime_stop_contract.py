import pytest

from runtime.runtime import ThalamusRuntime


class FakeBus:
    def __init__(self):
        self.closed = False

    async def close(self):
        self.closed = True


@pytest.mark.asyncio
async def test_regression_runtime_stop_uses_injected_bus_close_contract():
    fake_bus = FakeBus()
    runtime = ThalamusRuntime(bus=fake_bus)

    await runtime.stop()

    assert fake_bus.closed is True
