from jsonschema import validate


class EventValidator:

    def __init__(self, schema_store: dict):
        self.schema_store = schema_store

    def validate(self, schema_name: str, payload: dict):

        schema = self.schema_store[schema_name]

        validate(
            instance=payload,
            schema=schema
        )