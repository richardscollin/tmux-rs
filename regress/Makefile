TESTS!= echo *.sh

# Tests that fail in both upstream C tmux 3.6a and tmux-rs (outdated expected results)
SKIP_TESTS = combine-test.sh

TESTS := $(filter-out $(SKIP_TESTS),$(TESTS))

.PHONY: all $(TESTS)
.NOTPARALLEL: all $(TESTS)

all: $(TESTS)

$(TESTS):
	TEST_TMUX=../target/debug/tmux-rs sh $*.sh
	sleep 1
