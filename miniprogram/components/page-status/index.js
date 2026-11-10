Component({
  properties: { loading: Boolean, error: String, notice: String },
  methods: {
    retry() {
      this.triggerEvent("retry");
    },
  },
});
