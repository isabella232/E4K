# Developer documentation


## Debug environment
The debugging is made by building statically on the host and then mounting the executables in the pods.
### Install kubernetes minikube
Intall docker
```
wget https://packages.microsoft.com/config/ubuntu/18.04/multiarch/packages-microsoft-prod.deb -O packages-microsoft-prod.deb
sudo dpkg -i packages-microsoft-prod.deb
rm packages-microsoft-prod.deb

sudo apt-get update; \
  sudo apt-get install moby-engine
```

Install minikube
```
curl -LO https://storage.googleapis.com/minikube/releases/latest/minikube-linux-amd64
sudo install minikube-linux-amd64 /usr/local/bin/minikube
or
sudo chmod +x minikube-linux-amd64
sudo mv minikube-linux-amd64 /usr/local/bin/minikube
```

### Build code statically
Install openssl:
```
ln -s /usr/include/x86_64-linux-gnu/asm /usr/include/x86_64-linux-musl/asm && \
ln -s /usr/include/asm-generic /usr/include/x86_64-linux-musl/asm-generic && \
ln -s /usr/include/linux /usr/include/x86_64-linux-musl/linux

mkdir /musl


wget https://github.com/openssl/openssl/archive/OpenSSL_1_1_1f.tar.gz
tar zxvf OpenSSL_1_1_1f.tar.gz
cd openssl-OpenSSL_1_1_1f/

CC="musl-gcc -fPIE -pie" ./Configure no-shared no-async no-engine --prefix=/musl -DOPENSSL_NO_SECURE_MEMORY --openssldir=/musl/ssl linux-x86_64
make depend
make -j$(nproc)
make install
```

Export the following env var:
```
export PKG_CONFIG_ALLOW_CROSS=1
export OPENSSL_STATIC=true
export OPENSSL_DIR=/musl
```

Build:
cargo build --target=x86_64-unknown-linux-musl

Run:
export AZIOT_LOG=Debug
./executale_path

### Export executable in pods
With minikube, there are 2 mounts level:
1 - You need to mount the executable inside the minikube fake node (Here I mount the whole user repo):
```
minikube mount /home/azureuser:/home/azureuser
```
2 - You need to mound from the fake node inside your pod:
The exemple below mounts the binary as agentd
The config, extracted from the config map is also necessary and needs to be mounted: config
```
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: iotedge-spiffe-agent
  labels:
    app: iotedge-spiffe-agent
spec:
  selector:
    matchLabels:
      app: iotedge-spiffe-agent
  template:
    metadata:
      labels:
        app: iotedge-spiffe-agent
    spec:
      hostNetwork: true
      dnsPolicy: ClusterFirstWithHostNet    
      serviceAccountName: iotedge-spiffe-agent
      containers:
        - name: iotedge-spiffe-agent
          image:  ubuntu
          securityContext:
            privileged: true
            allowPrivilegeEscalation: true
          command: ["sleep"]
          args: ["1000000000"]
          volumeMounts:
            - name: working-repo
              mountPath: /agentd
            - name: config
              mountPath: /mnt/config    
      volumes:
        - name: working-repo
          hostPath:
            path: /home/azureuser/iot-k8s-identities/iot-edge-spiffe-agent/target/x86_64-unknown-linux-musl/debug/agentd
        - name: config
          configMap:
            name: iotedge-spiffe-agent 
```
