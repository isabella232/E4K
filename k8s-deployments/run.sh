#!/bin/bash
kubectl delete ClusterRole iotedge-spiffe-server-trust-role
kubectl delete ClusterRoleBinding iotedge-spiffe-server-trust-role-binding
kubectl delete serviceaccount iotedge-spiffe-server
kubectl delete configmap iotedge-spiffe-server
kubectl delete service iotedge-spiffe-server
kubectl delete deployment iotedge-spiffe-server


kubectl delete daemonset iotedge-spiffe-agent
kubectl delete configmap iotedge-spiffe-agent
kubectl delete serviceaccount iotedge-spiffe-agent


kubectl apply -f server/service-account.yaml
kubectl apply -f server/cluster-role.yaml
kubectl apply -f server/config-map.yaml 
kubectl apply -f server/service.yaml
kubectl apply -f server/deployment.yaml

kubectl apply -f agent/service-account.yaml
kubectl apply -f agent/cluster-role.yaml
kubectl apply -f agent/config-map.yaml 
kubectl apply -f agent/daemonset.yaml
