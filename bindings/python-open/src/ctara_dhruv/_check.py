"""Error handling for dhruv FFI calls.

Maps DhruvStatus codes from the C ABI to typed Python exceptions.
"""

from __future__ import annotations


class DhruvError(Exception):
    """Base exception for all dhruv FFI errors."""

    def __init__(self, code: int, message: str) -> None:
        self.code = code
        super().__init__(message)


class InvalidConfigError(DhruvError):
    """Status 1: Invalid engine configuration."""


class InvalidQueryError(DhruvError):
    """Status 2: Invalid query parameters."""


class KernelLoadError(DhruvError):
    """Status 3: Failed to load SPK kernel."""


class TimeConversionError(DhruvError):
    """Status 4: Time conversion failed."""


class UnsupportedQueryError(DhruvError):
    """Status 5: Query type not supported."""


class EpochOutOfRangeError(DhruvError):
    """Status 6: Epoch outside kernel coverage."""


class NullPointerError(DhruvError):
    """Status 7: Null pointer passed to FFI."""


class EopLoadError(DhruvError):
    """Status 8: Failed to load EOP data."""


class EopOutOfRangeError(DhruvError):
    """Status 9: EOP data out of range."""


class InvalidLocationError(DhruvError):
    """Status 10: Invalid geographic location."""


class NoConvergenceError(DhruvError):
    """Status 11: Iterative algorithm did not converge."""


class InvalidSearchConfigError(DhruvError):
    """Status 12: Invalid search configuration."""


class InvalidInputError(DhruvError):
    """Status 13: Invalid input parameter."""


class InternalError(DhruvError):
    """Status 255: Internal error."""


# Map DhruvStatus code -> exception class.
_STATUS_MAP: dict[int, type[DhruvError]] = {
    1: InvalidConfigError,
    2: InvalidQueryError,
    3: KernelLoadError,
    4: TimeConversionError,
    5: UnsupportedQueryError,
    6: EpochOutOfRangeError,
    7: NullPointerError,
    8: EopLoadError,
    9: EopOutOfRangeError,
    10: InvalidLocationError,
    11: NoConvergenceError,
    12: InvalidSearchConfigError,
    13: InvalidInputError,
    255: InternalError,
}


def check(status: int, context: str = "") -> None:
    """Raise a typed exception if *status* is non-zero.

    Args:
        status: DhruvStatus return code from a C ABI call.
        context: Optional description of the operation for the error message.

    Raises:
        DhruvError: (or a subclass) when *status* != 0.
    """
    if status == 0:
        return

    cls = _STATUS_MAP.get(status, DhruvError)
    name = cls.__name__
    msg = f"{name} (code {status})"
    if context:
        msg = f"{msg}: {context}"
    raise cls(status, msg)
