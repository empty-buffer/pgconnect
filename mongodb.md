# MongoDB / mongosh command reference

Quick reference for everyday work in [mongosh](https://www.mongodb.com/docs/mongodb-shell/). Commands run in the shell unless noted.

## Connect

```javascript
// URI (password special chars may need URL encoding)
mongosh "mongodb://user:pass@localhost:27017/mydb"

// Inside mongosh
mongosh --host localhost --port 27017 -u mongouser -p --authenticationDatabase admin
```

## Databases

```javascript
show dbs                    // list databases
use mydb                    // switch database (creates on first write)
db                          // current database name
db.getName()
db.dropDatabase()           // ⚠ destructive
```

## Collections

```javascript
show collections
db.createCollection("users")
db.users.drop()             // ⚠ destructive
db.users.renameCollection("accounts")
db.users.stats()
db.users.countDocuments()
db.users.estimatedDocumentCount()
```

## Insert

```javascript
db.users.insertOne({ name: "Ada", email: "ada@example.com", age: 30 })
db.users.insertMany([
  { name: "Bob", email: "bob@example.com" },
  { name: "Carol", email: "carol@example.com" }
])
```

## Find (select data)

```javascript
// All documents (limit output)
db.users.find()
db.users.find().limit(20).pretty()

// Filter
db.users.find({ status: "active" })
db.users.find({ age: { $gte: 18, $lte: 65 } })
db.users.find({ email: /example\.com$/ })   // regex

// Projection (fields to return)
db.users.find({}, { name: 1, email: 1, _id: 0 })

// One document
db.users.findOne({ _id: ObjectId("...") })

// Sort, skip, limit
db.users.find({ status: "active" }).sort({ createdAt: -1 }).skip(10).limit(25)

// Count
db.users.countDocuments({ status: "active" })
db.users.distinct("country")
```

### Query operators (common)

```javascript
{ field: { $eq: value } }      // equal (default: { field: value })
{ field: { $ne: value } }
{ field: { $in: [a, b] } }
{ field: { $nin: [a, b] } }
{ field: { $gt, $gte, $lt, $lte: n } }
{ field: { $exists: true } }
{ field: { $regex: "pattern", $options: "i" } }
{ $and: [ {...}, {...} ] }
{ $or:  [ {...}, {...} ] }
{ field: { $elemMatch: { k: v } } }   // arrays
```

## Update

```javascript
db.users.updateOne(
  { email: "ada@example.com" },
  { $set: { age: 31 }, $currentDate: { updatedAt: true } }
)

db.users.updateMany(
  { status: "pending" },
  { $set: { status: "active" } }
)

db.users.replaceOne({ _id: id }, { name: "Ada", email: "ada@example.com" })

// Upsert: insert if no match
db.users.updateOne(
  { email: "new@example.com" },
  { $set: { name: "New User" } },
  { upsert: true }
)

// Array / numeric operators
db.users.updateOne({ _id: id }, { $push: { tags: "vip" } })
db.users.updateOne({ _id: id }, { $pull: { tags: "vip" } })
db.users.updateOne({ _id: id }, { $inc: { loginCount: 1 } })
db.users.updateOne({ _id: id }, { $unset: { tempField: "" } })
```

## Delete

```javascript
db.users.deleteOne({ email: "spam@example.com" })
db.users.deleteMany({ status: "archived" })
```

## Indexes

```javascript
// List indexes
db.users.getIndexes()

// Single field
db.users.createIndex({ email: 1 })                    // ascending
db.users.createIndex({ createdAt: -1 })               // descending

// Compound
db.users.createIndex({ status: 1, createdAt: -1 })

// Unique
db.users.createIndex({ email: 1 }, { unique: true })

// Partial (only documents matching filter)
db.users.createIndex(
  { email: 1 },
  { unique: true, partialFilterExpression: { email: { $type: "string" } } }
)

// TTL (expire documents after seconds)
db.sessions.createIndex({ expiresAt: 1 }, { expireAfterSeconds: 0 })

// Text search
db.articles.createIndex({ title: "text", body: "text" })
db.articles.find({ $text: { $search: "mongodb index" } })

// Drop index
db.users.dropIndex("email_1")
db.users.dropIndexes()   // ⚠ all except _id
```

### Explain query plan

```javascript
db.users.find({ status: "active" }).explain("executionStats")
```

## Aggregation

```javascript
db.orders.aggregate([
  { $match: { status: "completed" } },
  { $group: { _id: "$customerId", total: { $sum: "$amount" }, count: { $sum: 1 } } },
  { $sort: { total: -1 } },
  { $limit: 10 }
])

// Lookup (join)
db.orders.aggregate([
  { $lookup: {
      from: "customers",
      localField: "customerId",
      foreignField: "_id",
      as: "customer"
  }},
  { $unwind: "$customer" }
])
```

## Schema / validation (optional)

```javascript
db.runCommand({
  collMod: "users",
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["email"],
      properties: {
        email: { bsonType: "string" },
        age: { bsonType: "int", minimum: 0 }
      }
    }
  },
  validationLevel: "moderate"
})
```

## Users & roles (admin)

```javascript
use admin
db.createUser({
  user: "appuser",
  pwd: "secret",
  roles: [{ role: "readWrite", db: "mydb" }]
})
db.getUsers()
db.dropUser("appuser")
```

## Server / diagnostics

```javascript
db.serverStatus()
db.stats()
db.currentOp()
db.killOp(opId)
rs.status()                 // replica set
sh.status()                 // sharded cluster
```

## Shell helpers

```javascript
help
db.help()
db.users.help()
load("/path/to/script.js")
exit                        // or Ctrl+D
```

## Docker (this repo)

From project root:

```bash
make compose_up
mongosh "mongodb://mongouser:mongopass@localhost:27017/admin"
```

Default credentials match `deploy/docker-compose.yml` (`mongouser` / `mongopass`).

## Tips

- `_id` is `ObjectId` by default; use `ObjectId("hexstring")` in filters.
- Prefer `.limit()` on large collections; `find()` without limit can flood the terminal.
- Create indexes **before** heavy production load; use `.explain()` to confirm index use.
- Authentication database is often `admin` for root users created via `MONGO_INITDB_ROOT_USERNAME`.
