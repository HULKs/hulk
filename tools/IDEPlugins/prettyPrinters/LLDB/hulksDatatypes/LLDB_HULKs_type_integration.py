# For the future world: Splitting this file into multiple ones ("for better overview and clean code")
# just did not work. LLDB just break. Ask @rkost

# Source for most of the commands: https://lldb.llvm.org/varformats.html

import re
import lldb
import os
import sys

sys.path.append(os.path.realpath(os.path.dirname(__file__)) + "/../eigen")

from LLDB_Eigen_Pretty_Printer import eigen_matrix_print_plain_option

# Executed during lldb init
def __lldb_init_module (debugger, dict):
    # add new data types to enable pretty printing on them
    debugger.HandleCommand("type summary add Pose -F \
                            LLDB_HULKs_type_integration.pose_print -w HULKs")
    #debugger.HandleCommand("type summary add -x \"^(const |)Parameter<.*>$\" -F \
    #                            LLDB_HULKs_type_integration.parameter_print -w HULKs")

    # reformat for better readability
    debugger.HandleCommand("type summary add float -F \
                                LLDB_HULKs_type_integration.reformat_helper -w HULKs.reformat")
    debugger.HandleCommand("type summary add double -F \
                                LLDB_HULKs_type_integration.reformat_helper -w HULKs.reformat")


    # activate the printers by category
    debugger.HandleCommand("type category enable HULKs")
    debugger.HandleCommand("type category enable HULKs.reformat")

    # Inline "pretty printing"
    debugger.HandleCommand("type summary add CycleInfo --summary-string \"startTime=${var.startTime.creationTime_}, "
                           "cycleTime=${var.cycleTime}, valid=${var.valid}\"")
    debugger.HandleCommand("type summary add Chronometer --summary-string \"startTime=${var.startTime_}\"")
    debugger.HandleCommand("type summary add -x \"^(const |)Parameter<.*>$\" --summary-string \"${var.value_}\"")


    # Setting some types to be printed inline
    debugger.HandleCommand("type summary add --inline-children -x \"^(const |)Range<.*>$\"")
    debugger.HandleCommand("type summary add --inline-children -x \"^(const |)TimePoint$\"")
    debugger.HandleCommand("type summary add --inline-children -x \"^std::vector<.*>$\"")
    debugger.HandleCommand("type summary add --inline-children -x \"^ActionCommand::(LED|Arm)$\"")


def pose_print(valobj, internal_dict):
    position_val = valobj.GetChildMemberWithName('position')
    orientation_val = valobj.GetChildMemberWithName('orientation')
    orientation = hulks_type_as_string(orientation_val, "float", internal_dict)
    return 'Position: ' + eigen_matrix_print_plain_option(position_val, internal_dict, plain_print=True) + \
           '; Orientation: ' + str(orientation)


def parameter_print(valobj, internal_dict):
    value_type_raw = valobj.GetType().GetName()
    # find basic type
    value_type = value_type_raw[value_type_raw.find('<') + 1: value_type_raw.find('>')]
    value_raw = valobj.GetChildMemberWithName('value_')

    value = hulks_type_as_string(raw_value=value_raw, typename=value_type, internal_dict=internal_dict)
    return value


def reformat_helper(valobj, internal_dict):
    return hulks_type_as_string(raw_value=valobj, typename=valobj.GetType().GetName(), internal_dict=internal_dict)


def hulks_type_as_string(raw_value, typename, internal_dict):
    if not (re.match('(const |)((unsigned |)(int|long)|uint8_t|uint16_t|uint32_t|uint64_t'
                     '|int8_t|int16_t|int32_t|int64_t)', typename) is None):
        return str(int(raw_value.GetValue()))
    elif not (re.match('(const |)(float|double)', typename) is None):
        return "%1.5e" % float(raw_value.GetValue())
    elif typename == "bool":
        return str(raw_value.GetValue())
    elif not (re.match('(const |)(Pose)', typename) is None):
        return pose_print(raw_value, internal_dict)
    elif not (typename.find("Eigen::") == -1):
        return eigen_matrix_print_plain_option(raw_value, internal_dict, plain_print=True)
    else:
        # unknown value type. Trying to return the raw value object
        # TODO try to let lldb print this value. Nobody knows how.
        # return str(raw_value.GetName())
        return "[HULKs type integration] Unknown value type: " + typename
