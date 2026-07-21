PWD:=$(shell pwd)
TOP:=$(PWD)/../..

include $(TOP)/platform.mk

INSTALL ?= install

PREFIX?=$(TOP)/inst
INSTALLDIR=$(PREFIX)/lib/VPI

.PHONY:	all
all: install

# -------------------------

# These get copied to the inst/lib/VPI area
HEADERS = bdpi.h vpi_user.h

# -------------------------

CFLAGS += -Wall \
	-Wmissing-prototypes \
	-Wpointer-arith \
	-Wshadow \
	-Wcast-qual \
	-Werror \
	-Wno-shadow \
	-g \
	-std=c99
ifneq ($(OSTYPE), Windows)
CFLAGS += -fPIC
endif
# vpi_user.h fails this check
#	-Wstrict-prototypes

COPT ?= -O3
CDEBUG ?=
CFLAGS += $(PROF) $(COPT) $(CDEBUG)

ifeq ($(OSTYPE), Windows)
# PE/COFF does not allow unresolved VPI symbols in a DLL.  Use Icarus's
# import libraries and produce an import library for generated VPI modules.
LIBBDPI=libbdpi.dll
LIBBDPI_IMPLIB=libbdpi.dll.a
LDFLAGS = $(shell iverilog-vpi --ldflags) -Wl,--out-implib,$(LIBBDPI_IMPLIB)
LDLIBS = $(shell iverilog-vpi --ldlibs)
else
ifeq ($(OSTYPE), Darwin)
# for Mac OS X, declare that the binary is dynamic and undefined references
# will be looked up later when the library is linked
LIBBDPI=libbdpi.dylib
LDFLAGS = -dynamiclib -Wl,-install_name,@rpath/$(LIBBDPI),-undefined,dynamic_lookup
else
LIBBDPI=libbdpi.so
LDFLAGS = -shared -Wl,-soname,$(LIBBDPI)
endif
endif

vpath %.c ../

%.o:%.c
	$(CC) -E -MM $< > $*.p
	cp $*.p $*.d; \
	sed -e 's/#.*//' -e 's/^[^:]*: *//' -e 's/ *\\$$//' \
	    -e '/^$$/ d' -e 's/$$/ :/' < $*.p >> $*.d; \
	rm -f $*.p
	$(CC) $(CFLAGS) -c -o $@ $<

# -------------------------

$(LIBBDPI): libbdpi.o
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $< $(LDLIBS)

# Static check of the header files before releasing them.
.PHONY: test-headers
test-headers:
	$(RM) test_hdr.c
	for hdr in $(HEADERS); do \
          echo "#include \"$$hdr\"" >> test_hdr.c ; \
	done
	$(CC) -c -o /dev/null test_hdr.c

.PHONY: install
install: $(HEADERS) $(LIBBDPI) test-headers
	$(INSTALL) -m 755 -d $(INSTALLDIR)
	$(INSTALL) -m 644 $(HEADERS) $(INSTALLDIR)
	$(INSTALL) -m 644 $(LIBBDPI) $(LIBBDPI_IMPLIB) $(INSTALLDIR)

.PHONY: clean
clean:
	$(RM) test_hdr.c
	$(RM) *.o *.d $(LIBBDPI) $(LIBBDPI_IMPLIB)

.PHONY: full_clean
full_clean: clean

TAGS:	*.h *.c
	etags *.h *.c

# include dependency information
DEPEND_OBJS = libbdpi.o

-include $(DEPEND_OBJS:.o=.d)

# -------------------------
