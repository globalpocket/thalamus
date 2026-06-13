from typing import Any
from typing import Dict
from typing import List
from typing import Optional
from typing import Union

from pydantic import BaseModel
from pydantic import Field


class RuntimeTaskAssignPayload(BaseModel):

    task_id: str

    agent_id: str

    input: Dict[str, Any] = Field(
        default_factory=dict
    )

    capabilities: List[str] = Field(
        default_factory=list
    )

    metadata: Dict[str, Any] = Field(
        default_factory=dict
    )


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


class RuntimeToolRequestPayload(BaseModel):

    request_id: str

    task_id: Optional[str] = None

    capability: str

    input: Dict[str, Any]

    agent_id: Optional[str] = None


class RuntimeToolResultPayload(BaseModel):

    request_id: str

    task_id: str

    status: str

    output: Optional[Any] = None

    error: Optional[str] = None


class RuntimeLLMRequestPayload(BaseModel):

    request_id: str

    task_id: Optional[str] = None

    prompt: str

    model: Optional[str] = None

    agent_id: Optional[str] = None


class RuntimeLLMResponsePayload(BaseModel):

    request_id: str

    task_id: str

    status: str

    text: Optional[str] = None

    model: str

    error: Optional[str] = None


class RuntimeAgentErrorPayload(BaseModel):

    agent_id: Optional[str] = None

    error: str

    task_id: Optional[str] = None



RuntimePayload = Union[
    RuntimeTaskAssignPayload,
    RuntimeTaskResultPayload,
    RuntimeAgentReadyPayload,
    RuntimeAgentExitPayload,
    RuntimeToolRequestPayload,
    RuntimeToolResultPayload,
    RuntimeLLMRequestPayload,
    RuntimeLLMResponsePayload,
    RuntimeAgentErrorPayload,
]


class RuntimeEvent(BaseModel):

    id: str

    type: str

    subject: str

    source: str

    timestamp: str

    schema: str

    payload: Dict[str, Any]

    correlation_id: Optional[str] = None

    causation_id: Optional[str] = None

    metadata: Dict[str, Any] = Field(
        default_factory=dict
    )
