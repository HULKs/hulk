#include <fstream>
#include <iostream>

#include "Definitions/keys.h"
#include "Tools/Storage/UniValue/UniValue2Json.hpp"

#include "MotionFile.hpp"


bool MotionFile::loadFromFile(const std::string& filename)
{
  Json::Reader reader;
  Json::Value root;
  std::ifstream f(filename);
  if (!f.is_open())
  {
    return false;
  }
  try
  {
    reader.parse(f, root);
  }
  catch (...)
  {
    return false;
  }
  fromValue(Uni::Converter::toUniValue(root));
  return true;
}

bool MotionFile::saveToFile(const std::string& filename) const
{
  std::ofstream f(filename);
  if (!f.is_open())
  {
    return false;
  }
  f << Uni::Converter::toJsonString(*this, true);
  return f.good();
}

bool MotionFile::verify() const
{
  if (header.time < 0)
  {
    return false;
  }
  if (header.joints.empty() || header.joints.size() > keys::joints::JOINTS_MAX)
  {
    return false;
  }
  for (auto joint : header.joints)
  {
    if (joint < 0 || joint >= keys::joints::JOINTS_MAX)
    {
      return false;
    }
  }
  if (header.version != "2.0")
  {
    return false;
  }
  for (auto& pos : position)
  {
    if (pos.time < 0)
    {
      return false;
    }
    if (pos.parameters.size() != header.joints.size())
    {
      return false;
    }
  }
  for (auto& stiff : stiffness)
  {
    if (stiff.time < 0)
    {
      return false;
    }
    if (stiff.parameters.size() != header.joints.size())
    {
      return false;
    }
  }
  return true;
}

void MotionFile::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::OBJECT);
  value["header"] << header;
  value["position"] << position;
  value["stiffness"] << stiffness;
}

void MotionFile::fromValue(const Uni::Value& value)
{
  value["header"] >> header;
  value["position"] >> position;
  value["stiffness"] >> stiffness;
}
