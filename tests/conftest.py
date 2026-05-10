import time

import pytest

from testcontainers.core.container import DockerContainer


@pytest.fixture(scope="session")
def nats_container():

    container = (
        DockerContainer("nats:latest")
        .with_exposed_ports(4222)
    )

    container.start()

    #
    # NATS boot wait
    #
    time.sleep(2)

    host = container.get_container_host_ip()
    port = container.get_exposed_port(4222)

    yield f"nats://{host}:{port}"

    container.stop()