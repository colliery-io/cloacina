"""Fire times of a five-field cron expression, for the checker (CLOACINA-T-1111).

The "Schedules" chapter computes when a schedule should have fired from its
own expression, not from what the server says. Standard library only.

Fields: minute, hour, day of month, month, day of week (0 or 7 is Sunday).
Each field takes `*`, a number, a range `a-b`, a step `*/n` or `a-b/n`, and
lists of those with `,`. Names (`MON`, `JAN`) are not read. When both the
day of month and the day of week are restricted, a day that matches either
one fires, as in cron.

Times are seconds since the epoch. A time zone is an IANA name; a fire time
is a minute whose local wall time matches. A local time that a change of
clocks skips never fires, and one that it repeats fires twice.
"""

from __future__ import annotations

from datetime import datetime, timezone

try:
    from zoneinfo import ZoneInfo
except ImportError:  # pragma: no cover - Python before 3.9
    ZoneInfo = None

UTC = timezone.utc  # noqa: UP017
BOUNDS = ((0, 59), (0, 23), (1, 31), (1, 12), (0, 7))
# Never look further than this for one fire time: a schedule that fires
# less than once in two years is not one the checker judges.
MAX_SCAN_MINUTES = 2 * 366 * 24 * 60


class CronError(ValueError):
    pass


def _field(text, low, high):
    values = set()
    for part in text.split(","):
        step = 1
        if "/" in part:
            part, step_text = part.split("/", 1)
            if not step_text.isdigit() or int(step_text) < 1:
                raise CronError(f"bad step in {text!r}")
            step = int(step_text)
        if part == "*":
            start, end = low, high
        elif "-" in part:
            start_text, end_text = part.split("-", 1)
            if not (start_text.isdigit() and end_text.isdigit()):
                raise CronError(f"bad range in {text!r}")
            start, end = int(start_text), int(end_text)
        elif part.isdigit():
            start = end = int(part)
            if step != 1:
                end = high
        else:
            raise CronError(f"cannot read {text!r}")
        if start < low or end > high or start > end:
            raise CronError(f"{text!r} is outside {low}-{high}")
        values.update(range(start, end + 1, step))
    return values


class Cron:
    def __init__(self, expression, tz="UTC"):
        fields = expression.split()
        if len(fields) != 5:
            raise CronError(f"{expression!r} does not have five fields")
        self.expression = expression
        self.minutes, self.hours, self.days, self.months, weekdays = (
            _field(text, low, high) for text, (low, high) in zip(fields, BOUNDS))
        self.weekdays = {0 if day == 7 else day for day in weekdays}
        self.any_day = fields[2] == "*"
        self.any_weekday = fields[4] == "*"
        if tz in (None, "", "UTC"):
            self.zone = UTC
        elif ZoneInfo is None:
            raise CronError("this Python has no zoneinfo")
        else:
            self.zone = ZoneInfo(tz)

    def matches(self, epoch):
        """True when the minute that holds `epoch` is a fire time."""
        local = datetime.fromtimestamp(int(epoch) // 60 * 60, tz=UTC).astimezone(self.zone)
        if local.minute not in self.minutes or local.hour not in self.hours:
            return False
        if local.month not in self.months:
            return False
        day = local.day in self.days
        weekday = (local.isoweekday() % 7) in self.weekdays
        if self.any_day and self.any_weekday:
            return True
        if self.any_day:
            return weekday
        if self.any_weekday:
            return day
        return day or weekday

    def next_after(self, epoch):
        """The first fire time strictly after `epoch`."""
        minute = int(epoch) // 60 * 60 + 60
        for _ in range(MAX_SCAN_MINUTES):
            if self.matches(minute):
                return float(minute)
            minute += 60
        raise CronError(f"{self.expression!r} has no fire time in two years")

    def previous_before(self, epoch):
        """The last fire time strictly before `epoch`."""
        minute = (int(epoch) + 59) // 60 * 60 - 60
        for _ in range(MAX_SCAN_MINUTES):
            if self.matches(minute):
                return float(minute)
            minute -= 60
        raise CronError(f"{self.expression!r} has no fire time in two years")

    def between(self, start, end):
        """Every fire time F with start <= F <= end, oldest first."""
        found = []
        minute = (int(start) + 59) // 60 * 60
        while minute <= end:
            if self.matches(minute):
                found.append(float(minute))
            minute += 60
        return found


def iso(epoch):
    """A fire time as the server writes it into a run's context."""
    return datetime.fromtimestamp(float(epoch), tz=UTC).isoformat()


def local_minute_for(utc_epoch, tz):
    """The minute of the hour, in `tz`, of a UTC time."""
    zone = UTC if tz in (None, "", "UTC") else ZoneInfo(tz)
    return datetime.fromtimestamp(float(utc_epoch), tz=UTC).astimezone(zone).minute
