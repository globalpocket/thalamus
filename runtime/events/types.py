from typing import Any
from typing import Dict
from typing import List
from typing import Optional
from typing import Union

from pydantic import BaseModel
from pydantic import Field


class RuntimeTaskAssignPayload(BaseModel):

    task_id: str

    objective: str


class RuntimeTaskResultPayload(BaseModel):

    task_id: str

    status: str

    summary: Optional[str] = None

    result: Optional[Dict[str, Any]] = None


class RuntimeAgentReadyPayload(BaseModel):

    agent_id: str

    capabilities: List[str] = Field(
        default_factory=list
    )


class RuntimeAgentExitPayload(BaseModel):

    agent_id: str

    reason: Optional[str] = None


RuntimePayload = Union[
    RuntimeTaskAssignPayload,
    RuntimeTaskResultPayload,
    RuntimeAgentReadyPayload,
    RuntimeAgentExitPayload,
]


class RuntimeEvent(BaseModel):

    type: str

    source: str

    payload: Dict[str, Any]