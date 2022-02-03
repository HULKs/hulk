#pragma once

#include <memory>
#include <typeindex>

class Option
{
public:
  virtual ~Option() = default;
  /**
   * @brief actionComplete states whether the action was completed successfully
   * @return true if the action was completed successfully
   */
  virtual bool actionComplete()
  {
    return false;
  }
  /**
   * @brief actionAborted states whether the action was aborted (e.g. converged
   * to an aborted state)
   * @return true if the action was aborted
   */
  virtual bool actionAborted()
  {
    return false;
  }

protected:
  // a pointer to the currently activeSubOption
  std::unique_ptr<Option> activeSubOption_;
  // the typeID
  std::type_index currentSubOptionTypeID_ = typeid(nullptr);

  /**
   * @brief callSubOptions calls a sub option of type O
   */
  template <typename O, typename... Args>
  void callSubOption(Args&... args)
  {
    // check whether we are running the same option as last time
    if (currentSubOptionTypeID_ != typeid(O))
    {
      currentSubOptionTypeID_ = typeid(O);
      // if we run a different one, destroy the old one (for resetting)
      activeSubOption_ = std::make_unique<O>();
    }
    O& subOption = static_cast<O&>(*activeSubOption_.get());
    // run the action and transition of the sub option
    subOption.transition(args...);
    subOption.action(args...);
  }
};
