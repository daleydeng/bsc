#include <cstdlib>

#include "rand32.h"

#ifdef _WIN32
namespace {

unsigned int random_state[31];
unsigned int random_front = 3;
unsigned int random_rear = 0;
bool random_initialized = false;

unsigned int next_random()
{
  random_state[random_front] += random_state[random_rear];
  unsigned int result = (random_state[random_front] >> 1) & 0x7fffffffu;
  random_front = (random_front + 1) % 31;
  random_rear = (random_rear + 1) % 31;
  return result;
}

void initialize_random()
{
  // Match the default-seeded glibc random() sequence used by upstream
  // simulation goldens. MinGW only provides the incompatible C rand().
  random_state[0] = 1;
  for (unsigned int i = 1; i < 31; ++i)
    random_state[i] = (unsigned int)
      ((16807ull * random_state[i - 1]) % 2147483647ull);

  random_initialized = true;
  for (unsigned int i = 0; i < 310; ++i)
    next_random();
}

}
#endif

extern "C" unsigned int rand32 ()
{
#ifdef _WIN32
  if (!random_initialized)
    initialize_random();
  return next_random();
#else
  unsigned int res;
  res = (unsigned int)random();
  return res;
#endif
}
