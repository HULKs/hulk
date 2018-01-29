#pragma once

#include <memory>

#include "Framework/Thread.hpp"

class Brain;
class RemoteControl;

class BrainThread : public Thread<BrainThread> {
public:
  /**
   * @brief BrainThread initializes members
   * @param data a reference to the thread data for brain
   */
  BrainThread(ThreadData& data);
  /**
   * @brief init initializes in the thread context
   * @return true iff successful
   */
  bool init();
  /**
   * @brief cycle executes the brain module manager
   */
  void cycle();
  /**
   * @brief getName returns a human readable name of the thread type
   * @return a string containing the name of the thread type
   */
  static std::string getName()
  {
    return "Brain";
  }
private:
  /// handle to the brain module manager
  std::shared_ptr<Brain> brain_;
  /// handle to the remote control module if loaded
  std::shared_ptr<RemoteControl> rcBrain_;
};
