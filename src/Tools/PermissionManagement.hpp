#pragma once

#include <cassert>

namespace PermissionManagement
{
  /**
   * @brief checkPermission checks whether an action is allowed to be performed by comparing the
   * bits of action and permission similar to unix file system permissions
   * (https://en.wikipedia.org/wiki/File_system_permissions#Numeric_notation). The permission
   * management requires all actions to be a unique power of two. The permission must be the sum of
   * actions that are permitted. Then each permission value is a unique combination of actions.
   * @param action the action to be checked
   * @param permission the permission
   * @return whether the action is allowed to be performed
   */
  inline bool checkPermission(const unsigned int action, const unsigned int permission)
  {
    // check if requested action is a power of two
    // (https://stackoverflow.com/questions/600293/how-to-check-if-a-number-is-a-power-of-2#600306)
    assert((!(action & (action - 1)) && action) && "action is not a power of two");
    // bits of action and permission are compared. If the action bit is set in the permission the
    // result is not zero
    return action & permission;
  }
} // namespace PermissionManagement
