TESTS!= echo *.sh

.PHONY: all $(TESTS)
.NOTPARALLEL: all $(TESTS)

all: $(TESTS)

$(TESTS):
	TEST_TMUX=../target/debug/tmux-rs sh $*.sh
	sleep 1
