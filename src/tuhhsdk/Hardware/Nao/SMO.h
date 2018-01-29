/**
 * @file SMO.h
 * @brief File providing enumerations, typedefs, and structs.
 * @author <a href="mailto:nicolas.riebesel@tuhh.de">Nicolas Riebesel</a>
 * @author <a href="mailto:oliver.tretau@tuhh.de">Oliver Tretau</a>
 *
 * This file provides several enumerations, typedefs, and strcts used within
 * the shared memory realization. Detailed descritpions are provided if
 * available.
 */

#ifndef SMO_H
#define SMO_H

#include <array>
#include <time.h>
#include <string>
#include <sys/time.h>
#include <linux/futex.h>
#include <sys/syscall.h>

#include <boost/interprocess/sync/interprocess_mutex.hpp>

#include "Definitions/keys.h"


/**
 * @namespace SMO
 * @brief The SMO namespace provides shared memory specifications.
 */
namespace SMO {
  extern const char* shmName;
  extern const size_t shmSize;
}

/**
 * @class TUHHSemaphore needed because glibc semaphores can't be used because of version conflicts
 * inspired by https://github.com/gallir/concurrencia/blob/master/futex/semaphore.c
 */
class TUHHSemaphore {
public:
  /**
   * @brief TUHHSemaphore creates a new semaphore
   * @param counter the initial counter
   */
  TUHHSemaphore(const int counter) :
    counter_(counter)
  {
  }
  /**
   * @brief getCounter returns the current value of the counter
   * @return the current value of the counter
   */
  int getCounter() const
  {
    // TODO: Test whether this is necessary/sufficient or return counter_ is always working
    int counter;
    asm volatile("movl %1, %0" : "=r" (counter) : "m" (counter_));
    return counter;
  }
  /**
   * @brief post increases the semaphore and wakes up if <=0
   */
  void post()
  {
    int counter = 1;
    // fetch and add inline assembly because aldebaran compiler does not have intrinsics
    asm volatile("lock xaddl %0, %1" : "+r" (counter), "+m" (counter_) : : "memory");
    // counter still has the old value (which is one less), thus <0 comparison
    if (counter < 0) {
      while (syscall(SYS_futex, &futex_, FUTEX_WAKE, 1, 0, 0, 0) < 1) {
        sched_yield();
      }
    }
  }
  /**
   * @brief wait decreases the semaphore and waits if <0
   */
  void wait()
  {
    int counter = -1;
    // fetch and add inline assembly because aldebaran compiler does not have intrinsics
    asm volatile("lock xaddl %0, %1" : "+r" (counter), "+m" (counter_) : : "memory");
    // counter still has the old value (which is one more), thus <=0 comparison
    if (counter <= 0) {
      syscall(SYS_futex, &futex_, FUTEX_WAIT, futex_, 0, 0, 0);
    }
  }
private:
  /// the futex (an arbitrary integer number)
  int futex_;
  /// the counter
  int counter_;
};

/**
 * @struct SharedBlock
 * @brief The SharedBlock struct is the essential construct provided
 *
 * This struct contains all possible joint data, aliases, callbacks and sensor
 * values that need to be communicated between the tuhhALModule and
 * tuhhNao.
 */
struct SharedBlock {
  SharedBlock() :
    chestButtonPressed(false),
    chestButtonDoublePressed(false),
    semaphore(0),
    newAngles(false),
    newStiffnesses(false),
    newLEDs(false),
    newSonar(false)
  {}

  std::array<float, keys::joints::JOINTS_MAX> commandAngles;
  std::array<float, keys::joints::JOINTS_MAX> commandStiffnesses;
  std::array<float, keys::led::CHEST_MAX + 2 * keys::led::EAR_MAX + 2 * keys::led::EYE_MAX + keys::led::HEAD_MAX + 2 * keys::led::FOOT_MAX> commandLEDs;
  float commandSonar;

  // joint information
  std::array<float, keys::joints::JOINTS_MAX> jointSensor;      ///< Sensor values of all joints
  std::array<float, keys::joints::JOINTS_MAX> jointCurrent;     ///< Current values of all joints
  std::array<float, keys::joints::JOINTS_MAX> jointTemperature; ///< Temperature values of all joints
  std::array<float, keys::joints::JOINTS_MAX> jointStatus; ///< Status values of all joints

  // sensor information
  std::array<float, keys::sensor::SWITCH_MAX> switches; ///< All switch key values
  std::array<float, keys::sensor::IMU_MAX> imu;         ///< All imu key values
  std::array<float, keys::sensor::FSR_MAX> fsrLeft;     ///< All left Force Sensitive Resistors (FSR) key values
  std::array<float, keys::sensor::FSR_MAX> fsrRight;    ///< All right Force Sensitive Resistors (FSR) key values
  std::array<float, keys::sensor::SONAR_MAX> sonar;     ///< All sonar key values
  std::array<float, keys::sensor::BATTERY_MAX> battery; ///< All battery key values

  // nao information
  std::array<char[64], keys::naoinfos::NAOINFO_MAX> naoInfoKey; ///< Custom information key values

  int64_t time;                   ///< Real time when sensor values were sampled (time since system clock epoch in nanoseconds)
  bool chestButtonPressed;
  bool chestButtonDoublePressed;

  /**
   * @typedef SharedBlock::mutex_t
   * @brief This typedef allows an easily readable way to perform scoped locks
   *        on the access mutex.
   */
  typedef boost::interprocess::interprocess_mutex mutex_t;
  mutex_t accessMutex; ///< Mutex providing access to shared block

  TUHHSemaphore semaphore;
  bool newAngles;
  bool newStiffnesses;
  bool newLEDs;
  bool newSonar;

};

#endif // SMO_H
