outdir ?= .

srpm:
	mkdir -p $(outdir)
	tar --exclude-vcs --exclude='./target' --exclude='./src-tauri/target' --exclude='./node_modules' \
		--transform 's,^\.,nodus-studio-0.1.0,' -czf $(outdir)/nodus-studio-0.1.0.tar.gz .
	rpmbuild -bs \
		--define "_sourcedir $(outdir)" \
		--define "_srcrpmdir $(outdir)" \
		packaging/rpm/nodus-studio.spec
