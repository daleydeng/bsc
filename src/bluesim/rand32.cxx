#include <cstdlib>

#include "rand32.h"

extern "C" unsigned int rand32 ()
{
#ifdef _WIN32
  // MinGW does not provide POSIX random().  Combine the 15-bit values from
  // the standard C generator so that the Bluesim primitive still returns
  // all 32 result bits.
  unsigned int res = (unsigned int)rand();
  res = (res << 15) ^ (unsigned int)rand();
  return (res << 2) ^ ((unsigned int)rand() & 0x3u);
#else
  unsigned int res;
  res = (unsigned int)random();
  return res;
#endif
}
