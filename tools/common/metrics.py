# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Wraps the dogstatsd client because the plain ``statsd`` package has no tag
support.
"""

import optparse
import os
from typing import Optional, Sequence

from datadog import initialize, statsd

# Env vars that ``add_metric_options`` also reads, so a script picks up the
# same configuration from flags or env.
HOST_ENV_VAR = "SYNC_STATSD_HOST"
PORT_ENV_VAR = "SYNC_STATSD_PORT"


class Metrics:
    """Send statsd counters, gauges, and timings with optional tags.

    ``namespace`` is prepended to every metric name by the underlying client. A
    metric emitted with ``namespace="wibble"`` and label ``"errors"`` arrives
    as ``wibble.errors``.
    """

    def __init__(
        self,
        namespace: str = "",
        host: Optional[str] = None,
        port: Optional[int | str] = None,
    ):
        """Configure the process-global dogstatsd client.

        ``host`` and ``port`` fall back to ``SYNC_STATSD_HOST`` and
        ``SYNC_STATSD_PORT``.  Both unset is localhost.
        """
        if host is None:
            host = os.environ.get(HOST_ENV_VAR)
        if port is None:
            port = os.environ.get(PORT_ENV_VAR)

        self.prefix = namespace
        initialize(
            namespace=namespace,
            statsd_namespace=namespace,
            statsd_host=host,
            statsd_port=port,
        )

    @classmethod
    def from_opts(cls, opts: optparse.Values, namespace: str = "") -> "Metrics":
        """Build from an ``optparse`` namespace populated by ``add_metric_options``."""
        return cls(
            namespace=namespace,
            host=getattr(opts, "metric_host", None),
            port=getattr(opts, "metric_port", None),
        )

    def incr(
        self, label: str, value: int = 1, tags: Optional[Sequence[str]] = None
    ) -> None:
        """Increment a statsd counter with the given label and optional tags."""
        statsd.increment(label, value=value, tags=tags)

    def gauge(
        self, label: str, value: float, tags: Optional[Sequence[str]] = None
    ) -> None:
        """Record a point-in-time gauge value."""
        statsd.gauge(label, value, tags=tags)

    def timing(
        self, label: str, value_ms: float, tags: Optional[Sequence[str]] = None
    ) -> None:
        """Record a timing value in milliseconds."""
        statsd.timing(label, value_ms, tags=tags)


def add_metric_options(parser: optparse.OptionParser) -> None:
    """Add generic metric related options to an OptionParser"""
    parser.add_option(
        "",
        "--metric_host",
        default=os.environ.get(HOST_ENV_VAR),
        help="Metric host name",
    )
    parser.add_option(
        "",
        "--metric_port",
        default=os.environ.get(PORT_ENV_VAR),
        help="Metric host port",
    )
