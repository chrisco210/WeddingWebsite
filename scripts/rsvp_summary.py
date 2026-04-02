#!/usr/bin/env python3
"""
RSVP Summary — reads from the wedding-rsvp DynamoDB table and prints a report.

Usage:
    python3 scripts/rsvp_summary.py [--table TABLE_NAME] [--region REGION] [--format text|csv]

AWS credentials are picked up from the environment / ~/.aws/credentials as usual.
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

GUESTS_CSV = (
    Path(__file__).parent.parent / "server" / "rsvp" / "src" / "guests.csv"
)


# ── Data model ────────────────────────────────────────────────────────────────

@dataclass
class GuestRow:
    party_id: str
    party_display_name: str
    guest_name: str
    status: str          # "attending", "declined", "no_response"
    dietary_restrictions: str


@dataclass
class Summary:
    total_invited: int
    total_attending: int
    total_declined: int
    total_no_response: int
    parties_responded: int
    total_parties: int
    guests: List[GuestRow] = field(default_factory=list)


# ── Load the authoritative guest list from CSV ────────────────────────────────

def load_guest_list(path: Path) -> dict:
    """Return {party_id: {id, display_name, guests: [str]}}."""
    parties = {}
    with open(path, newline="", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            row = next(csv.reader(io.StringIO(line)))
            if len(row) < 3:
                continue
            party_id = row[0].strip()
            display_name = row[1].strip()
            guest_name = row[2].strip()
            if party_id not in parties:
                parties[party_id] = {
                    "id": party_id,
                    "display_name": display_name,
                    "guests": [],
                }
            parties[party_id]["guests"].append(guest_name)
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

        for guest_name in party["guests"]:
            if rsvp is None:
                status = "no_response"
                dietary = ""
            else:
                response = rsvp.get(guest_name)
                if response is None:
                    status = "no_response"
                    dietary = ""
                else:
                    status = "attending" if response.get("attending") else "declined"
                    dietary = (response.get("dietary_restrictions") or "").strip()

            guests.append(GuestRow(
                party_id=party_id,
                party_display_name=party["display_name"],
                guest_name=guest_name,
                status=status,
                dietary_restrictions=dietary,
            ))

    total_invited = len(guests)
    total_attending = sum(1 for g in guests if g.status == "attending")
    total_declined = sum(1 for g in guests if g.status == "declined")
    total_no_response = sum(1 for g in guests if g.status == "no_response")

    return Summary(
        total_invited=total_invited,
        total_attending=total_attending,
        total_declined=total_declined,
        total_no_response=total_no_response,
        parties_responded=parties_responded,
        total_parties=len(parties),
        guests=guests,
    )


# ── Output formatters ─────────────────────────────────────────────────────────

def output_text(summary: Summary) -> None:
    attending = [g for g in summary.guests if g.status == "attending"]
    declined = [g for g in summary.guests if g.status == "declined"]
    no_response = [g for g in summary.guests if g.status == "no_response"]
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
    writer.writerow(["party_id", "party_display_name", "guest_name", "status", "dietary_restrictions"])
    for g in summary.guests:
        writer.writerow([g.party_id, g.party_display_name, g.guest_name, g.status, g.dietary_restrictions])


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

    parties = load_guest_list(GUESTS_CSV)
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
