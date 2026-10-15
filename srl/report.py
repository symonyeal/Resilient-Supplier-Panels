"""Findings tables, rebuilt from saved result JSON only (never from a live run)."""


def count(r):
    """A run's output count; a stopped run gives a lower bound, never a count."""
    return str(r["count"]) if r["status"] == "complete" else f"≥{r['count']} ({r['status']})"


def essential(rec):
    """Essential suppliers come from the matrix witness (V \\ {v} infeasible), so they need
    no complete enumeration."""
    if rec["essential"] is None:
        return "— (infeasible)"
    return f"{len(rec['essential'])} (matrix witness)"
