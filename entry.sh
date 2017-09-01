#!/bin/sh

# Taken from https://github.com/docker-library/mysql/issues/47#issuecomment-147397851
# Traps SIGINT and SIGTERM signals and forwards them to the child process as SIGTERM signals
asyncRun() {
    "$@" &
    pid="$!"
    trap "echo 'Stopping PID $pid'; kill -SIGTERM $pid" SIGINT SIGTERM

    # A signal emitted while waiting will make the wait command return code > 128
    # Let's wrap it in a loop that doesn't end before the process is indeed stopped
    while kill -0 $pid > /dev/null 2>&1; do
        wait
    done
}
asyncRun $@