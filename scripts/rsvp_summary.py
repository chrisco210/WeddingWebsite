#!/usr/bin/env python3
"""
RSVP Summary — reads from the wedding-rsvp DynamoDB table and prints a report.

Usage:
    python3 scripts/rsvp_summary.py [--table TABLE_NAME] [--region REGION] [--format text|csv]

AWS credentials are picked up from the environment / ~/.aws/credentials as usual.

The guest list is read from rsvp/tf/guests.csv. Each line has the format
    party_display_name,guest_name,guest_alias
(the alias column may be empty). Parties are grouped by display name, and each
party's id is the xxh3_64 hash of its (trimmed) display name — this must match
the id the Rust backend (rsvp/src/guest_list.rs) uses as the DynamoDB key.

Welcome-dinner invites are read from rsvp/tf/welcome_party.txt (one display name
per line). RSVP responses (rsvp/src/handler.rs::GuestRsvp) carry an optional
`attending_welcome_dinner` flag for invited parties.
"""

import argparse
import csv
import io
import json
import os
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional

import boto3

try:
    import xxhash
except ImportError:
    print(
        "ERROR: the 'xxhash' package is required (party ids are xxh3_64 hashes).\n"
        "       Install it with:  python3 -m pip install xxhash",
        file=sys.stderr,
    )
    sys.exit(1)

TF_DIR = Path(__file__).parent.parent / "rsvp" / "tf"
GUESTS_CSV = TF_DIR / "guests.csv"
WELCOME_PARTY_TXT = TF_DIR / "welcome_party.txt"


# ── Data model ────────────────────────────────────────────────────────────────

@dataclass
class GuestRow:
    party_id: str
    party_display_name: str
    guest_name: str
    status: str          # "attending", "declined", "no_response"
    dietary_restrictions: str
    welcome_dinner_invite: bool
    # "attending", "declined", "no_response", or "n/a" (party not invited)
    welcome_dinner_status: str


@dataclass
class Summary:
    total_invited: int
    total_attending: int
    total_declined: int
    total_no_response: int
    parties_responded: int
    total_parties: int
    welcome_invited: int
    welcome_attending: int
    welcome_declined: int
    welcome_no_response: int
    guests: List[GuestRow] = field(default_factory=list)


# ── Load the authoritative guest list from CSV ────────────────────────────────

def party_id_for(display_name: str) -> str:
    """Match the Rust backend: xxh3_64 of the trimmed display name, as a decimal string."""
    return str(xxhash.xxh3_64_intdigest(display_name.encode("utf-8")))


def load_welcome_party(path: Path) -> set:
    """Return the set of party display names invited to the welcome dinner."""
    if not path.exists():
        return set()
    with open(path, encoding="utf-8") as fh:
        return {line.strip() for line in fh if line.strip()}


def load_guest_list(path: Path, welcome_names: set) -> dict:
    """Return {party_id: {id, display_name, guests: [str], welcome_dinner_invite}}."""
    parties = {}
    with open(path, newline="", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            row = next(csv.reader(io.StringIO(line)))
            # Format: display_name, guest_name, [alias]
            if len(row) < 2:
                continue
            display_name = row[0].strip()
            guest_name = row[1].strip()
            if not display_name or not guest_name:
                continue
            party_id = party_id_for(display_name)
            party = parties.setdefault(party_id, {
                "id": party_id,
                "display_name": display_name,
                "guests": [],
                "welcome_dinner_invite": display_name in welcome_names,
            })
            party["guests"].append(guest_name)
    return parties


# ── Scan DynamoDB ─────────────────────────────────────────────────────────────

def scan_table(table_name: str, region: Optional[str]) -> list:
    """Return all items from the table."""
    kwargs = {}
    if region:
        kwargs["region_name"] = region
    ddb = boto3.client("dynamodb", **kwargs)

    items = []
    paginator = ddb.get_paginator("scan")
    for page in paginator.paginate(TableName=table_name):
        items.extend(page.get("Items", []))
    return items


def parse_rsvp_item(item: dict) -> tuple:
    """Return (party_id, [GuestRsvp dicts]) from a raw DynamoDB item."""
    party_id = item["party_id"]["S"]
    rsvp_data = json.loads(item.get("rsvp_data", {}).get("S", "[]"))
    return party_id, rsvp_data


# ── Build the summary data structure ─────────────────────────────────────────

def build_summary(parties: dict, rsvp_items: list) -> Summary:
    rsvp_by_party = {}
    for item in rsvp_items:
        party_id, responses = parse_rsvp_item(item)
        rsvp_by_party[party_id] = {r["name"]: r for r in responses}

    guests = []
    parties_responded = 0

    for party_id, party in sorted(parties.items(), key=lambda kv: kv[1]["display_name"]):
        rsvp = rsvp_by_party.get(party_id)
        if rsvp is not None:
            parties_responded += 1
        invited_to_welcome = party["welcome_dinner_invite"]

        for guest_name in party["guests"]:
            response = rsvp.get(guest_name) if rsvp is not None else None
            if response is None:
                status = "no_response"
                dietary = ""
                welcome_status = "n/a" if not invited_to_welcome else "no_response"
            else:
                status = "attending" if response.get("attending") else "declined"
                dietary = (response.get("dietary_restrictions") or "").strip()
                if not invited_to_welcome:
                    welcome_status = "n/a"
                else:
                    attending_welcome = response.get("attending_welcome_dinner")
                    if attending_welcome is None:
                        welcome_status = "no_response"
                    else:
                        welcome_status = "attending" if attending_welcome else "declined"

            guests.append(GuestRow(
                party_id=party_id,
                party_display_name=party["display_name"],
                guest_name=guest_name,
                status=status,
                dietary_restrictions=dietary,
                welcome_dinner_invite=invited_to_welcome,
                welcome_dinner_status=welcome_status,
            ))

    total_invited = len(guests)
    total_attending = sum(1 for g in guests if g.status == "attending")
    total_declined = sum(1 for g in guests if g.status == "declined")
    total_no_response = sum(1 for g in guests if g.status == "no_response")

    welcome_guests = [g for g in guests if g.welcome_dinner_invite]
    welcome_invited = len(welcome_guests)
    welcome_attending = sum(1 for g in welcome_guests if g.welcome_dinner_status == "attending")
    welcome_declined = sum(1 for g in welcome_guests if g.welcome_dinner_status == "declined")
    welcome_no_response = sum(1 for g in welcome_guests if g.welcome_dinner_status == "no_response")

    return Summary(
        total_invited=total_invited,
        total_attending=total_attending,
        total_declined=total_declined,
        total_no_response=total_no_response,
        parties_responded=parties_responded,
        total_parties=len(parties),
        welcome_invited=welcome_invited,
        welcome_attending=welcome_attending,
        welcome_declined=welcome_declined,
        welcome_no_response=welcome_no_response,
        guests=guests,
    )


# ── Output formatters ─────────────────────────────────────────────────────────

def output_text(summary: Summary) -> None:
    attending = [g for g in summary.guests if g.status == "attending"]
    declined = [g for g in summary.guests if g.status == "declined"]
    no_response = [g for g in summary.guests if g.status == "no_response"]
    welcome_attending = [g for g in summary.guests if g.welcome_dinner_status == "attending"]
    dietary = [(g.guest_name, g.dietary_restrictions) for g in summary.guests if g.dietary_restrictions]

    print("=" * 60)
    print("  RSVP SUMMARY")
    print("=" * 60)
    print(f"  Total invited:          {summary.total_invited}")
    print(f"  Parties responded:      {summary.parties_responded} / {summary.total_parties}")
    print(f"  Attending:              {summary.total_attending}")
    print(f"  Declined:               {summary.total_declined}")
    print(f"  No response yet:        {summary.total_no_response}")
    print()
    print(f"  Welcome dinner invited: {summary.welcome_invited}")
    print(f"  Welcome attending:      {summary.welcome_attending}")
    print(f"  Welcome declined:       {summary.welcome_declined}")
    print(f"  Welcome no response:    {summary.welcome_no_response}")
    print()

    print("-" * 60)
    print("  ATTENDING")
    print("-" * 60)
    current_party = None
    for g in attending:
        if g.party_display_name != current_party:
            print(f"  {g.party_display_name}")
            current_party = g.party_display_name
        suffix = f"  [diet: {g.dietary_restrictions}]" if g.dietary_restrictions else ""
        print(f"    + {g.guest_name}{suffix}")
    print()

    print("-" * 60)
    print("  DECLINED")
    print("-" * 60)
    current_party = None
    for g in declined:
        if g.party_display_name != current_party:
            print(f"  {g.party_display_name}")
            current_party = g.party_display_name
        print(f"    - {g.guest_name}")
    print()

    print("-" * 60)
    print("  NO RESPONSE YET")
    print("-" * 60)
    current_party = None
    for g in no_response:
        if g.party_display_name != current_party:
            print(f"  {g.party_display_name}")
            current_party = g.party_display_name
        print(f"    ? {g.guest_name}")
    print()

    print("-" * 60)
    print("  WELCOME DINNER — ATTENDING")
    print("-" * 60)
    current_party = None
    for g in welcome_attending:
        if g.party_display_name != current_party:
            print(f"  {g.party_display_name}")
            current_party = g.party_display_name
        print(f"    + {g.guest_name}")
    print()

    if dietary:
        print("-" * 60)
        print("  DIETARY RESTRICTIONS")
        print("-" * 60)
        for name, dr in sorted(dietary, key=lambda x: x[0]):
            print(f"  {name}: {dr}")
        print()

    print("=" * 60)


def output_csv(summary: Summary) -> None:
    writer = csv.writer(sys.stdout)
    writer.writerow([
        "party_id", "party_display_name", "guest_name", "status",
        "dietary_restrictions", "welcome_dinner_invite", "welcome_dinner_status",
    ])
    for g in summary.guests:
        writer.writerow([
            g.party_id, g.party_display_name, g.guest_name, g.status,
            g.dietary_restrictions, g.welcome_dinner_invite, g.welcome_dinner_status,
        ])


# ── Entry point ───────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(description="Print RSVP summary from DynamoDB")
    parser.add_argument(
        "--table",
        default=os.environ.get("TABLE_NAME", "wedding-rsvp"),
        help="DynamoDB table name (default: wedding-rsvp)",
    )
    parser.add_argument(
        "--region",
        default=os.environ.get("AWS_REGION"),
        help="AWS region (falls back to AWS_REGION env var / profile default)",
    )
    parser.add_argument(
        "--format",
        choices=["text", "csv"],
        default="text",
        help="Output format: text (default) or csv",
    )
    args = parser.parse_args()

    if not GUESTS_CSV.exists():
        print(f"ERROR: guests.csv not found at {GUESTS_CSV}", file=sys.stderr)
        sys.exit(1)

    welcome_names = load_welcome_party(WELCOME_PARTY_TXT)
    if not WELCOME_PARTY_TXT.exists():
        print(f"WARNING: welcome_party.txt not found at {WELCOME_PARTY_TXT}", file=sys.stderr)

    parties = load_guest_list(GUESTS_CSV, welcome_names)
    print(f"Loaded {len(parties)} parties from guests.csv", file=sys.stderr)

    print(f"Scanning DynamoDB table '{args.table}'...", file=sys.stderr)
    items = scan_table(args.table, args.region)
    print(f"Found {len(items)} RSVP submissions", file=sys.stderr)
    print(file=sys.stderr)

    summary = build_summary(parties, items)

    if args.format == "csv":
        output_csv(summary)
    else:
        output_text(summary)


if __name__ == "__main__":
    main()
