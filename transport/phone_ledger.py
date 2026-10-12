"""Durable command admission records; never executes or retries a command.

The service must validate authority before reserve and must not dispatch until
reserve returns True. A failed write is a failed admission. Recovered records
are evidence of uncertainty, not a work queue. Terminal records are immutable.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3
import stat
import threading
import uuid


class LedgerError(ValueError):
    pass


def _identity(value):
    if not isinstance(value, str) or not re.fullmatch('[0-9a-f]{32}', value):
        raise LedgerError('Invalid operation identity')


class Ledger:
    def __init__(self, directory, limit=1024):
        self.directory = Path(directory)
        self.limit = limit
        self.epoch = uuid.uuid4().hex
        self.mutex = threading.RLock()
        self.db = None
        self.lock_fd = None
        self.directory.mkdir(mode=0o700, parents=True, exist_ok=True)
        info = self.directory.lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
            raise LedgerError('Private state directory required')
        try:
            self.lock_fd = os.open(self.directory / 'owner.lock',
                                   os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
            fcntl.flock(self.lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            path = self.directory / 'operations.sqlite'
            fd = os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
            try:
                info = os.fstat(fd)
                if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
                    raise LedgerError('Private ledger file required')
                os.fsync(fd)
            finally:
                os.close(fd)
            # Persist creation of the database entry before any admission.
            fd = os.open(self.directory, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(fd)
            finally:
                os.close(fd)
            self.db = sqlite3.connect(path, timeout=2, check_same_thread=False)
            self.db.execute('PRAGMA journal_mode=DELETE')
            self.db.execute('PRAGMA synchronous=FULL')
            with self.db:
                self.db.execute('CREATE TABLE IF NOT EXISTS operations ('
                                'epoch TEXT NOT NULL, id TEXT NOT NULL, digest TEXT NOT NULL, '
                                'state TEXT NOT NULL, result TEXT, PRIMARY KEY(epoch,id))')
                self.db.execute('CREATE TABLE IF NOT EXISTS acknowledgments ('
                                'epoch TEXT NOT NULL, id TEXT NOT NULL, PRIMARY KEY(epoch,id))')
                self.db.execute("UPDATE operations SET state='unknown' WHERE state='pending'")
        except Exception as error:
            self.close()
            if isinstance(error, BlockingIOError):
                raise LedgerError('Service already owns this ledger') from error
            raise

    def reserve(self, epoch, operation_id, command):
        _identity(epoch)
        _identity(operation_id)
        if epoch != self.epoch:
            raise LedgerError('Expired service epoch')
        if not isinstance(command, dict):
            raise LedgerError('Command object required')
        try:
            data = json.dumps(command, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()
        except (ValueError, TypeError, RecursionError) as error:
            raise LedgerError('Invalid command') from error
        if len(data) > 16384:
            raise LedgerError('Command too large')
        digest = hashlib.sha256(data).hexdigest()
        with self.mutex, self.db:
            row = self.db.execute('SELECT digest FROM operations WHERE epoch=? AND id=?',
                                  (epoch, operation_id)).fetchone()
            if row:
                if row[0] != digest:
                    raise LedgerError('Operation identity reused with different arguments')
                return False
            if self.db.execute('SELECT count(*) FROM operations').fetchone()[0] >= self.limit:
                raise LedgerError('Command ledger full')
            self.db.execute('INSERT INTO operations VALUES (?,?,?,?,NULL)',
                            (epoch, operation_id, digest, 'pending'))
        # Exiting the transaction commits before the caller can dispatch.
        return True

    def result(self, epoch, operation_id):
        _identity(epoch)
        _identity(operation_id)
        with self.mutex:
            row = self.db.execute('SELECT state,result FROM operations WHERE epoch=? AND id=?',
                                  (epoch, operation_id)).fetchone()
        return None if row is None else {'state': row[0], 'result': json.loads(row[1]) if row[1] else None}

    def unresolved(self):
        """Discover retained uncertainty, never a queue to execute or replay.

        The ledger itself is bounded. Return at most 64 identities so discovery
        fits one IPC frame, and explicitly indicate that further records exist.
        No raw command payload or digest is exposed.
        """
        items = []
        with self.mutex:
            rows = self.db.execute('SELECT o.epoch,o.id,o.state,o.result FROM operations o '
                                   'LEFT JOIN acknowledgments a ON o.epoch=a.epoch AND o.id=a.id '
                                   'WHERE a.id IS NULL ORDER BY o.rowid')
            for epoch, identity, state, encoded in rows:
                result = json.loads(encoded) if encoded else None
                if state == 'finished' and result and result.get('outcome') != 'unknown':
                    continue
                if len(items) == 64:
                    return {'items': items, 'more': True}
                items.append({'epoch': epoch, 'id': identity, 'state': state, 'result': result})
        return {'items': items, 'more': False}

    def acknowledge(self, epoch, operation_id):
        """Durably accept uncertainty without changing outcome or dispatch rights."""
        _identity(epoch)
        _identity(operation_id)
        with self.mutex, self.db:
            operation = self.result(epoch, operation_id)
            if operation is None or operation['state'] == 'pending':
                raise LedgerError('Terminal operation required')
            if operation['state'] == 'finished' and operation['result']['outcome'] != 'unknown':
                return False
            self.db.execute('INSERT OR IGNORE INTO acknowledgments VALUES (?,?)', (epoch, operation_id))
        return True

    def finish(self, epoch, operation_id, result):
        _identity(epoch)
        _identity(operation_id)
        if epoch != self.epoch:
            raise LedgerError('Expired service epoch')
        if result is not None:
            if not isinstance(result, dict) or set(result) != {'outcome', 'reason'} or any(
                not isinstance(v, str) or not re.fullmatch('[a-z_]{1,64}', v) for v in result.values()
            ):
                raise LedgerError('Bounded outcome and reason required')
        encoded = json.dumps(result, sort_keys=True) if result is not None else None
        with self.mutex, self.db:
            cursor = self.db.execute("UPDATE operations SET state=?,result=? WHERE epoch=? AND id=? AND state='pending'",
                                     ('finished' if result is not None else 'unknown', encoded, epoch, operation_id))
            if cursor.rowcount != 1:
                raise LedgerError('No pending operation')

    def close(self):
        with self.mutex:
            if self.db is not None:
                self.db.close()
                self.db = None
            if self.lock_fd is not None:
                os.close(self.lock_fd)
                self.lock_fd = None
