#include "Framework/Database.hpp"

Database::~Database()
{
  for (auto it : dataMap_)
  {
    delete it.second.data;
  }
}

void Database::reset(const std::type_index& type)
{
  auto it = dataMap_.find(type);
  // Imported data must not be resetted since if no new message arrives it is assumed that the state
  // persists.
  if (it != dataMap_.end() && !it->second.imported)
  {
    it->second.data->reset();
  }
}

void Database::send()
{
  for (auto* sender : senders_)
  {
    for (const auto& dataType : sender->getRequested())
    {
      auto it = dataMap_.find(dataType);
      // Imported data must not be sent even if it is requested because it will be sent by the
      // original provider.
      if (it == dataMap_.end() || it->second.imported)
      {
        continue;
      }
      Message message;
      message.type = it->first;
      message.data = it->second.data->copy();
      sender->send(message);
    }
  }
}

void Database::receive()
{
  for (auto* const receiver : receivers_)
  {
    Message message;
    while (receiver->receive(message))
    {
      auto it = dataMap_.find(message.type);
      if (it == dataMap_.end())
      {
        throw std::runtime_error("DataType has no entry in Database when receive is called");
      }
      if (!it->second.imported)
      {
        throw std::runtime_error("DataType is not imported but received");
      }
      message.data->copy(it->second.data);
      delete message.data;
    }
  }
}

void Database::request(const std::type_index& type)
{
  for (auto* const receiver : receivers_)
  {
    receiver->request(type);
  }
  auto it = dataMap_.find(type);
  if (it == dataMap_.end())
  {
    throw std::runtime_error("DataType has no entry in Database when request is called");
  }
  it->second.imported = true;
}

void Database::produce(const std::type_index& type)
{
  for (auto* sender : senders_)
  {
    sender->produce(type);
  }
}

void Database::addSender(Sender* sender)
{
  senders_.push_back(sender);
}

void Database::addReceiver(Receiver* receiver)
{
  receivers_.push_back(receiver);
}
