#[macro_export]
macro_rules! define_callback {
    ($scope:vis $cb_name:ident, $cb_trait:ty) => {
        paste::paste! {
            $scope trait [<$cb_name Fn>]: $cb_trait + 'static {}

            impl<S> [<$cb_name Fn>] for S where S: $cb_trait + 'static {}

            #[derive(Clone)]
            $scope struct $cb_name {
                $scope closure: std::rc::Rc<dyn [<$cb_name Fn>]>
            }

            impl std::fmt::Debug for $cb_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{} {{ closure: _ }}", stringify!($cb_name))
                }
            }

            impl $cb_name {
                $scope fn new<F>(cb: F) -> $cb_name
                where
                    F: [<$cb_name Fn>],
                {
                    Self {
                       closure: std::rc::Rc::new(cb)
                    }
                }
            }

            impl From<std::rc::Rc<dyn [<$cb_name Fn>]>> for $cb_name {
                fn from(f: std::rc::Rc<dyn [<$cb_name Fn>]>) -> $cb_name {
                    Self {
                       closure: f
                    }
                }
            }

            impl<T> From<T> for $cb_name
            where
                T: [<$cb_name Fn>]
            {
                fn from(f: T) -> $cb_name {
                    Self::new(f)
                }
            }

            impl std::ops::Deref for $cb_name {
                type Target = dyn [<$cb_name Fn>];

                fn deref(&self) -> &Self::Target {
                    &*self.closure
                }
            }
        }
    };
}
