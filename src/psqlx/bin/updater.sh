cp src/psqlx/src/psqlx.h external/psql/src/include/

# Avoid applying the patch if it has already been applied
if cat external/psql/src/bin/psql/command.c | grep -q has_command_ext; then
  echo "has_command_ext found, skipping."
else
  # Insert content at line 319 in command.c
  sed '319r src/psqlx/src/command.c' external/psql/src/bin/psql/command.c > temp.txt && mv temp.txt external/psql/src/bin/psql/command.c
fi

# Check if Makefile already has the pattern
if cat external/psql/src/bin/psql/Makefile | grep -q -e "-L../../../src/bin/psqlx/target/release -lpsqlx"; then
  echo "Pattern found in Makefile, skipping."
else
  # Replace line 50 in Makefile
  sed '50c\
	$(CC) $(CFLAGS) $(OBJS) $(LDFLAGS) $(LDFLAGS_EX) $(LIBS) -o $@$(X) -L../../../../../target/release -lpsqlx -ldl -lpthread
    ' \
  external/psql/src/bin/psql/Makefile > temp.txt && mv temp.txt external/psql/src/bin/psql/Makefile
  
  # Change psql to psqlx on line 67 of Makefile
  sed '68s/$(bindir)\/psql$(X)/$(bindir)\/psqlx$(X)/' external/psql/src/bin/psql/Makefile > temp.txt && mv temp.txt external/psql/src/bin/psql/Makefile
fi