# Interface

## 全局数据


- NEIGHBORS : 使用Hashmap 存放每个interface的neighbor，key都是接口的ipv4 address。

- INTERFACES : 机器中所有的接口。

- INTERFACES_BY_NAME : 使用接口名作为接口的键值。

- HANDLERS : 接口的异步协程。

- TRANSMISSION : 接口用于数据传输的收发端。


## 初始化

- 初始化所有的全局数据

