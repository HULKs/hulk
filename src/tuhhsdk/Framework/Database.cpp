#include "Database.hpp"

Database::~Database()
{
  for (auto it : data_map_) {
    delete it.second.data;
  }
}

void Database::reset(const std::type_index& type)
{
  auto it = data_map_.find(type);
  // Imported data must not be resetted since if no new message arrives it is assumed that the state persists.
  if (it != data_map_.end() && !it->second.imported) {
    it->second.data->reset();
  }
}

void Database::send()
{
  for (auto sender : senders_) {
    for (auto& data_type : sender->getRequested()) {
      auto it = data_map_.find(data_type);
      // Imported data must not be sent even if it is requested because it will be sent by the original provider.
      if (it == data_map_.end() || it->second.imported) {
        continue;
      }
      Message msg;
      msg.type = it->first;
      msg.data = it->second.data->copy();
      sender->send(msg);
    }
  }
}

void Database::receive()
{
  for (auto receiver : receivers_) {
    Message msg;
    while (receiver->receive(msg)) {
      auto it = data_map_.find(msg.type);
      if (it == data_map_.end()) {
        throw std::runtime_error("DataType has no entry in Database when receive is called!");
      }
      if (!it->second.imported) {
        throw std::runtime_error("DataType is not imported but received!");
      }
      msg.data->copy(it->second.data);
      delete msg.data;
    }
  }
}

void Database::request(const std::type_index& type)
{
  for (auto receiver : receivers_) {
    receiver->request(type);
  }
  auto it = data_map_.find(type);
  if (it == data_map_.end()) {
    throw std::runtime_error("DataType has no entry in Database when request is called!");
  }
  it->second.imported = true;
}

void Database::addSender(Sender* sender)
{
  senders_.push_back(sender);
}

void Database::addReceiver(Receiver* receiver)
{
  receivers_.push_back(receiver);
}
