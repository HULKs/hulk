#pragma once

#include <memory>

#include "Framework/Thread.hpp"

class Motion;

class MotionThread : public Thread<MotionThread> {
public:
  /**
   * @brief MotionThread initializes members
   * @param data a reference to the thread data for motion
   */
  MotionThread(ThreadData& data);
  /**
   * @brief init initializes in the thread context
   * @return true iff successful
   */
  bool init();
  /**
   * @brief cycle executes the motion module manager
   */
  void cycle();
  /**
   * @brief getName returns a human readable name of the thread type
   * @return a string containing the name of the thread type
   */
  static std::string getName()
  {
    return "Motion";
  }
private:
  /// handle to the motion module manager
  std::shared_ptr<Motion> motion_;
};
