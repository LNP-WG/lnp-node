---
name: Bug report
about: Let's improve software together
title: ''
labels: bug
assignees: dr-orlovsky

---

**Checklist**
- [ ] Make sure you are using the latest code (`git checkout master && git pull origin`)
- [ ] Compile only with rust nightly: `rustup default nightly && rustup update nightly && cargo test --all --all-features -v`
- [ ] Try to do `cargo update`
- [ ] Try to remove `target` directory

**Classify the bug**
Put `x` in the boxes below:
- [ ] Build issue
- [ ] Test failing
- [ ] Runtime panic
- [ ] Incorrect results
- [ ] Unexpected/undocumented behaviour / 

**Describe the problem**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Operating system
2. Rust compiler version
3. Did you 

**Expected behavior**
A clear and concise description of what you expected to happen.

**Logs**
Please copy and paste content of `rustup default nightly && cargo test --all --all-features -v` in a block below right after "console" line:
```console
```

**Additional context**
Add any other context about the problem here.

**Other links**
Please provide links and references to the affected repositories, code samples etc.
