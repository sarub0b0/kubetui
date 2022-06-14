use std::{io::stdout, time::Duration};

use crossterm::{
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tui::{backend::CrosstermBackend, layout::Rect, widgets::Widget, Terminal};

use kubetui::{
    signal::signal_handler,
    tui_wrapper::{
        event::EventResult,
        widget::{LiteralItem, RenderTrait, Text, WidgetTrait},
        Window, WindowEvent,
    },
};

// {{{ sample string
const DATA: &str = r#"
あいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえお
012345あいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえお
あいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえおあいうえお012345
apiVersion: v1
kind: Pod
metadata:
  annotations:
    kubeadm.kubernetes.io/kube-apiserver.advertise-address.endpoint: 192.168.65.4:6443
    kubernetes.io/config.hash: f76ea91a200a6b1cfe31c7a114460aac
    kubernetes.io/config.mirror: f76ea91a200a6b1cfe31c7a114460aac
    kubernetes.io/config.seen: "2022-05-15T00:52:21.390862747Z"
    kubernetes.io/config.source: file
    seccomp.security.alpha.kubernetes.io/pod: runtime/default
  creationTimestamp: "2022-05-15T00:52:26Z"
  labels:
    component: kube-apiserver
    tier: control-plane
  name: kube-apiserver-docker-desktop
  namespace: kube-system
  ownerReferences:
  - apiVersion: v1
    controller: true
    kind: Node
    name: docker-desktop
    uid: 1479614d-162f-44a6-9d9b-b56eaab73a6b
  resourceVersion: "870849"
  uid: 8328dc22-45e8-4061-81ac-bfc92576b9c6
spec:
  containers:
  - command:
    - kube-apiserver
    - --advertise-address=192.168.65.4
    - --allow-privileged=true
    - --authorization-mode=Node,RBAC
    - --client-ca-file=/run/config/pki/ca.crt
    - --enable-admission-plugins=NodeRestriction
    - --enable-bootstrap-token-auth=true
    - --etcd-cafile=/run/config/pki/etcd/ca.crt
    - --etcd-certfile=/run/config/pki/apiserver-etcd-client.crt
    - --etcd-keyfile=/run/config/pki/apiserver-etcd-client.key
    - --etcd-servers=https://127.0.0.1:2379
    - --kubelet-client-certificate=/run/config/pki/apiserver-kubelet-client.crt
    - --kubelet-client-key=/run/config/pki/apiserver-kubelet-client.key
    - --kubelet-preferred-address-types=InternalIP,ExternalIP,Hostname
    - --proxy-client-cert-file=/run/config/pki/front-proxy-client.crt
    - --proxy-client-key-file=/run/config/pki/front-proxy-client.key
    - --requestheader-allowed-names=front-proxy-client
    - --requestheader-client-ca-file=/run/config/pki/front-proxy-ca.crt
    - --requestheader-extra-headers-prefix=X-Remote-Extra-
    - --requestheader-group-headers=X-Remote-Group
    - --requestheader-username-headers=X-Remote-User
    - --secure-port=6443
    - --service-account-issuer=https://kubernetes.default.svc.cluster.local
    - --service-account-key-file=/run/config/pki/sa.pub
    - --service-account-signing-key-file=/run/config/pki/sa.key
    - --service-cluster-ip-range=10.96.0.0/12
    - --tls-cert-file=/run/config/pki/apiserver.crt
    - --tls-private-key-file=/run/config/pki/apiserver.key
    - --watch-cache=false
    image: k8s.gcr.io/kube-apiserver:v1.24.0
    imagePullPolicy: IfNotPresent
    livenessProbe:
      failureThreshold: 8
      httpGet:
        host: 192.168.65.4
        path: /livez
        port: 6443
        scheme: HTTPS
      initialDelaySeconds: 10
      periodSeconds: 10
      successThreshold: 1
      timeoutSeconds: 15
    name: kube-apiserver
    readinessProbe:
      failureThreshold: 3
      httpGet:
        host: 192.168.65.4
        path: /readyz
        port: 6443
        scheme: HTTPS
      periodSeconds: 1
      successThreshold: 1
      timeoutSeconds: 15
    resources:
      requests:
        cpu: 250m
    startupProbe:
      failureThreshold: 24
      httpGet:
        host: 192.168.65.4
        path: /livez
        port: 6443
        scheme: HTTPS
      initialDelaySeconds: 10
      periodSeconds: 10
      successThreshold: 1
      timeoutSeconds: 15
    terminationMessagePath: /dev/termination-log
    terminationMessagePolicy: File
    volumeMounts:
    - mountPath: /etc/ssl/certs
      name: ca-certs
      readOnly: true
    - mountPath: /etc/ca-certificates
      name: etc-ca-certificates
      readOnly: true
    - mountPath: /run/config/pki
      name: k8s-certs
      readOnly: true
    - mountPath: /usr/local/share/ca-certificates
      name: usr-local-share-ca-certificates
      readOnly: true
    - mountPath: /usr/share/ca-certificates
      name: usr-share-ca-certificates
      readOnly: true
  dnsPolicy: ClusterFirst
  enableServiceLinks: true
  hostNetwork: true
  nodeName: docker-desktop
  preemptionPolicy: PreemptLowerPriority
  priority: 2000001000
  priorityClassName: system-node-critical
  restartPolicy: Always
  schedulerName: default-scheduler
  securityContext:
    seccompProfile:
      type: RuntimeDefault
  terminationGracePeriodSeconds: 30
  tolerations:
  - effect: NoExecute
    operator: Exists
  volumes:
  - hostPath:
      path: /etc/ssl/certs
      type: DirectoryOrCreate
    name: ca-certs
  - hostPath:
      path: /etc/ca-certificates
      type: DirectoryOrCreate
    name: etc-ca-certificates
  - hostPath:
      path: /run/config/pki
      type: DirectoryOrCreate
    name: k8s-certs
  - hostPath:
      path: /usr/local/share/ca-certificates
      type: DirectoryOrCreate
    name: usr-local-share-ca-certificates
  - hostPath:
      path: /usr/share/ca-certificates
      type: DirectoryOrCreate
    name: usr-share-ca-certificates
status:
  conditions:
  - lastProbeTime: null
    lastTransitionTime: "2022-06-04T14:25:17Z"
    status: "True"
    type: Initialized
  - lastProbeTime: null
    lastTransitionTime: "2022-06-04T14:25:36Z"
    status: "True"
    type: Ready
  - lastProbeTime: null
    lastTransitionTime: "2022-06-04T14:25:36Z"
    status: "True"
    type: ContainersReady
  - lastProbeTime: null
    lastTransitionTime: "2022-06-04T14:25:17Z"
    status: "True"
    type: PodScheduled
  containerStatuses:
  - containerID: docker://5c50df15f0756d7e7ee87dc60888b3519dd2d75af08fc2319f49f26e878a4a7a
    image: k8s.gcr.io/kube-apiserver:v1.24.0
    imageID: docker://sha256:529072250ccc6301cb341cd7359c9641b94a6f837f86f82b1223a59bb0712e64
    lastState:
      terminated:
        containerID: docker://277ce0397db0efa177407710c413cb702bd0dca456009dc999c775cd5ad659f6
        exitCode: 255
        finishedAt: "2022-06-04T14:25:09Z"
        reason: Error
        startedAt: "2022-06-04T10:24:31Z"
    name: kube-apiserver
    ready: true
    restartCount: 15
    started: true
    state:
      running:
        startedAt: "2022-06-04T14:25:18Z"
  hostIP: 192.168.65.4
  phase: Running
  podIP: 192.168.65.4
  podIPs:
  - ip: 192.168.65.4
  qosClass: Burstable
  startTime: "2022-06-04T14:25:17Z"
"#;
// }}}

use std::io::Write;
fn main() {
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        disable_raw_mode().unwrap();
        execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();

        std::io::stderr()
            .write_all(b"\x1b[31mPanic! disable raw mode\x1b[39m")
            .unwrap();

        default_hook(info);
    }));

    signal_handler();

    enable_raw_mode().unwrap();

    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.clear().unwrap();

    let item = DATA.lines().map(|l| l.into()).collect::<Vec<String>>();

    let mut text = Text::builder()
        .items(item.clone())
        .wrap()
        .action(KeyCode::Char('q'), |_| {
            EventResult::Window(WindowEvent::CloseWindow)
        })
        .action(KeyCode::Esc, |_| {
            EventResult::Window(WindowEvent::CloseWindow)
        })
        .build();

    text.update_chunk(terminal.size().unwrap());

    loop {
        terminal
            .draw(|f| {
                text.render(f, true);
            })
            .unwrap();

        if poll(Duration::from_millis(200)).unwrap() {
            let result = {
                match read() {
                    Ok(ev) => match ev {
                        Event::Key(key) => match text.on_key_event(key) {
                            EventResult::Window(w) => w,
                            EventResult::Callback(Some(cb)) => {
                                if let EventResult::Window(ev) = (cb)(&mut Window::default()) {
                                    ev
                                } else {
                                    WindowEvent::Continue
                                }
                            }
                            _ => WindowEvent::Continue,
                        },

                        Event::Mouse(ev) => {
                            text.on_mouse_event(ev);

                            WindowEvent::Continue
                        }
                        Event::Resize(w, h) => {
                            text.update_chunk(Rect::new(0, 0, w, h));

                            WindowEvent::Continue
                        }
                    },
                    Err(_) => break,
                }
            };

            if matches!(result, WindowEvent::CloseWindow) {
                break;
            }
        }
    }

    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
}
