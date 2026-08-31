export function Toast({ message }: { message: string }) {
  return (
    <div className="k-well k-toast">
      <span className="k-toast-dot" />
      {message}
    </div>
  )
}
