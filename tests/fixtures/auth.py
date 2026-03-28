from typing import Optional


class AuthService:
    """Handles authentication and session management."""

    def __init__(self, session_store):
        self.session_store = session_store

    def validate_token(self, token: str) -> bool:
        if not token:
            return False
        return self._verify_signature(token)

    @staticmethod
    def refresh_session(session_id: str) -> None:
        # Refresh the session expiry
        session = session_store.get(session_id)
        if session is not None:
            session.extend()

    def _verify_signature(self, token: str) -> bool:
        return True  # simplified


def standalone_function(x: int) -> int:
    return x * 2
