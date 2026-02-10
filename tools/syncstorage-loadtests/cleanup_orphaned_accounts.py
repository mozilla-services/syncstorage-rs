#!/usr/bin/env python3
"""
Cleanup orphaned accounts from interrupted tests.

Usage:
    python cleanup_orphaned_accounts.py
"""

import os
import json
from fxa.core import Client
from fxa.errors import ClientError, ServerError

ACCT_TRACKING_FILE = os.path.join(os.path.dirname(__file__), '.accounts_tracking.json')
FXA_API_HOST = os.environ.get("FXA_API_HOST", "https://api-accounts.stage.mozaws.net")


def load_tracked_accounts():
    if not os.path.exists(ACCT_TRACKING_FILE):
        return []

    try:
        with open(ACCT_TRACKING_FILE, 'r') as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError) as e:
        print(f"Warning: Could not load tracking file: {e}")
        return []


def save_tracked_accounts(accounts):
    try:
        if not accounts:
            if os.path.exists(ACCT_TRACKING_FILE):
                os.remove(ACCT_TRACKING_FILE)
        else:
            with open(ACCT_TRACKING_FILE, 'w') as f:
                json.dump(accounts, f, indent=2)
    except IOError as e:
        print(f"Warning: Could not save tracking file: {e}")
        raise

def remove_account_from_tracking(email):
    accounts = load_tracked_accounts()
    accounts = [acc for acc in accounts if acc['email'] != email]
    save_tracked_accounts(accounts)


def cleanup_account(client, account):
    email = account['email']
    password = account['password']

    try:
        client.destroy_account(email, password)
        print(f"  ✓ Deleted: {email}")
        return True
    except (ServerError, ClientError) as ex:
        print(f"  ✗ Delete failed: {email} - {ex}")
        return False
    except Exception as ex:
        print(f"  ✗ Delete error: {email} - {ex}")
        return False


def cleanup_all_accounts():
    accounts = load_tracked_accounts()

    if not accounts:
        print("No accounts to clean up.")
        return 0, 0

    print(f"\nFound {len(accounts)} accounts")
    print("\nAttempting to delete accounts...\n")

    client = Client(FXA_API_HOST)
    successful = 0
    failed = 0

    for account in accounts:
        if cleanup_account(client, account):
            successful += 1
            remove_account_from_tracking(account['email'])
        else:
            failed += 1

    print(f"\nResults: {successful} deleted, {failed} failed")

    return successful, failed


def main():
    cleanup_all_accounts()


if __name__ == "__main__":
    main()
